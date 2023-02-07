use anyhow::{anyhow, Result};

use penumbra_chain::genesis;
use penumbra_storage::Storage;
use tendermint::abci::{self, ConsensusRequest as Request, ConsensusResponse as Response};
use tokio::sync::mpsc;
use tracing::{instrument, Instrument};

use super::Message;
use crate::App;

pub struct Worker {
    queue: mpsc::Receiver<Message>,
    storage: Storage,
    app: App,
}

impl Worker {
    #[instrument(skip(storage, queue), name = "consensus::Worker::new")]
    pub async fn new(storage: Storage, queue: mpsc::Receiver<Message>) -> Result<Self> {
        let app = App::new(storage.latest_state());

        Ok(Self {
            queue,
            storage,
            app,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(Message {
            req,
            rsp_sender,
            span,
        }) = self.queue.recv().await
        {
            // The send only fails if the receiver was dropped, which happens
            // if the caller didn't propagate the message back to tendermint
            // for some reason -- but that's not our problem.
            let _ = rsp_sender.send(match req {
                Request::InitChain(init_chain) => Response::InitChain(
                    self.init_chain(init_chain)
                        .instrument(span)
                        .await
                        .expect("init_chain must succeed"),
                ),
                Request::BeginBlock(begin_block) => Response::BeginBlock(
                    self.begin_block(begin_block)
                        .instrument(span)
                        .await
                        .expect("begin_block must succeed"),
                ),
                Request::DeliverTx(deliver_tx) => {
                    Response::DeliverTx(self.deliver_tx(deliver_tx).instrument(span.clone()).await)
                }
                Request::EndBlock(end_block) => Response::EndBlock(
                    self.end_block(end_block)
                        .instrument(span)
                        .await
                        .expect("end_block must succeed"),
                ),
                Request::Commit => Response::Commit(
                    self.commit()
                        .instrument(span)
                        .await
                        .expect("commit must succeed"),
                ),
            });
        }
        Ok(())
    }

    /// Initializes the chain based on the genesis data.
    ///
    /// The genesis data is provided by tendermint, and is used to initialize
    /// the database.
    async fn init_chain(
        &mut self,
        init_chain: abci::request::InitChain,
    ) -> Result<abci::response::InitChain> {
        tracing::info!(?init_chain);
        // Note that errors cannot be handled in InitChain, the application must crash.
        let app_state: genesis::AppState = serde_json::from_slice(&init_chain.app_state_bytes)
            .expect("can parse app_state in genesis file");

        // Check that we haven't got a duplicated InitChain message for some reason:
        if self.storage.latest_version() != u64::MAX {
            return Err(anyhow!("database already initialized"));
        }
        self.app.init_chain(&app_state).await;

        // Extract the Tendermint validators from the app state
        //
        // NOTE: we ignore the validators passed to InitChain.validators, and instead expect them
        // to be provided inside the initial app genesis state (`GenesisAppState`). Returning those
        // validators in InitChain::Response tells Tendermint that they are the initial validator
        // set. See https://docs.tendermint.com/master/spec/abci/abci.html#initchain
        let validators = self.app.tendermint_validator_updates();

        // Note: we defer committing until the first EndBlock. Normally we'd generate an app_hash
        // by committing, but we don't have state to preserve yet, so we'll return a default blank
        // value to Tendermint in the InitChain Reponse.
        let app_hash: Vec<u8> = vec![255u8; 32]
            .try_into()
            .expect("can initialize a default app_hash for initchain response");

        tracing::info!(
            consensus_params = ?init_chain.consensus_params,
            ?validators,
            app_hash = ?app_hash,
            "finished init_chain"
        );

        Ok(abci::response::InitChain {
            consensus_params: Some(init_chain.consensus_params),
            validators,
            app_hash: app_hash.into(),
        })
    }

    async fn begin_block(
        &mut self,
        begin_block: abci::request::BeginBlock,
    ) -> Result<abci::response::BeginBlock> {
        // We don't need to print the block height, because it will already be
        // included in the span modeling the abci request handling.
        tracing::info!(time = ?begin_block.header.time, "beginning block");
        let events = self.app.begin_block(&begin_block).await;
        Ok(abci::response::BeginBlock { events })
    }

    async fn deliver_tx(
        &mut self,
        deliver_tx: abci::request::DeliverTx,
    ) -> abci::response::DeliverTx {
        // Unlike the other messages, DeliverTx is fallible, so
        // inspect the response to report errors.
        let rsp = self.app.deliver_tx_bytes(deliver_tx.tx.as_ref()).await;

        match rsp {
            Ok(events) => {
                tracing::info!("deliver_tx succeeded");
                abci::response::DeliverTx {
                    events,
                    ..Default::default()
                }
            }
            Err(e) => {
                tracing::info!(?e, "deliver_tx failed");
                abci::response::DeliverTx {
                    code: 1,
                    // Use the alternate format specifier to include the chain of error causes.
                    log: format!("{:#}", e),
                    ..Default::default()
                }
            }
        }
    }

    async fn end_block(
        &mut self,
        end_block: abci::request::EndBlock,
    ) -> Result<abci::response::EndBlock> {
        let events = self.app.end_block(&end_block).await;

        // Set `tm_validator_updates` to the complete set of
        // validators and voting power. This must be the last step performed,
        // after all voting power calculations and validator state transitions have
        // been completed.
        let validator_updates = self.app.tendermint_validator_updates();

        tracing::debug!(
            ?validator_updates,
            "sending validator updates to tendermint"
        );

        Ok(abci::response::EndBlock {
            validator_updates,
            consensus_param_updates: None,
            events,
        })
    }

    async fn commit(&mut self) -> Result<abci::response::Commit> {
        let app_hash = self.app.commit(self.storage.clone()).await;
        tracing::info!(?app_hash, "committed block");

        Ok(abci::response::Commit {
            data: app_hash.0.to_vec().into(),
            retain_height: 0u32.into(),
        })
    }
}
