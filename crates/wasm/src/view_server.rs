use crate::error::WasmResult;
use crate::note_record::SpendableNoteRecord;
use crate::storage::IndexedDBStorage;
use crate::swap_record::SwapRecord;
use penumbra_asset::asset::{DenomMetadata, Id};
use penumbra_compact_block::{CompactBlock, StatePayload};
use penumbra_dex::lp::position::Position;
use penumbra_dex::lp::LpNft;
use penumbra_keys::FullViewingKey;
use penumbra_sct::Nullifier;
use penumbra_shielded_pool::note;
use penumbra_tct as tct;
use penumbra_tct::Witness::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::Error;
use std::convert::TryInto;
use std::{collections::BTreeMap, str::FromStr};
use tct::storage::{StoreCommitment, StoreHash, StoredPosition, Updates};
use tct::{Forgotten, Tree};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredTree {
    last_position: Option<StoredPosition>,
    last_forgotten: Option<Forgotten>,
    hashes: Vec<StoreHash>,
    commitments: Vec<StoreCommitment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanBlockResult {
    height: u64,
    nct_updates: Updates,
    new_notes: Vec<SpendableNoteRecord>,
    new_swaps: Vec<SwapRecord>,
}

impl ScanBlockResult {
    pub fn new(
        height: u64,
        nct_updates: Updates,
        new_notes: Vec<SpendableNoteRecord>,
        new_swaps: Vec<SwapRecord>,
    ) -> ScanBlockResult {
        Self {
            height,
            nct_updates,
            new_notes,
            new_swaps,
        }
    }
}

#[wasm_bindgen]
pub struct ViewServer {
    latest_height: u64,
    epoch_duration: u64,
    fvk: FullViewingKey,
    notes: BTreeMap<note::StateCommitment, SpendableNoteRecord>,
    notes_by_nullifier: BTreeMap<Nullifier, SpendableNoteRecord>,
    swaps: BTreeMap<tct::StateCommitment, SwapRecord>,
    denoms: BTreeMap<Id, DenomMetadata>,
    nct: Tree,
    storage: IndexedDBStorage,
}

#[wasm_bindgen]
impl ViewServer {
    #[wasm_bindgen(constructor)]
    pub async fn new(
        full_viewing_key: &str,
        epoch_duration: u64,
        stored_tree: JsValue,
    ) -> WasmResult<ViewServer> {
        let fvk = FullViewingKey::from_str(full_viewing_key)?;
        let stored_tree: StoredTree = serde_wasm_bindgen::from_value(stored_tree)?;
        let tree = load_tree(stored_tree);
        let view_server = Self {
            latest_height: u64::MAX,
            fvk,
            epoch_duration,
            notes: Default::default(),
            notes_by_nullifier: Default::default(),
            denoms: Default::default(),
            nct: tree,
            swaps: Default::default(),
            storage: IndexedDBStorage::new().await?,
        };
        Ok(view_server)
    }

    /// Scans block for notes, swaps
    /// This method does not return SCT updates (nct_updates).
    /// Although we can make it do if we want to save SCT updates after each blob is processed
    /// Arguments:
    ///     compact_block: `v1alpha1::CompactBlock`
    /// Returns: `ScanBlockResult`
    #[wasm_bindgen]
    pub async fn scan_block(&mut self, compact_block: JsValue) -> Result<JsValue, Error> {
        let result = self.scan_block_inner(compact_block).await?;
        serde_wasm_bindgen::to_value(&result)
    }

    /// get SCT state updates
    /// This method is necessary because we save SCT updates to indexedDB once every 1000 blocks
    /// rather than after each block
    /// Arguments:
    ///     last_position: `Option<StoredPosition>`
    ///     last_forgotten: `Option<Forgotten>`
    /// Returns: `ScanBlockResult`
    #[wasm_bindgen]
    pub fn get_updates(
        &mut self,
        last_position: JsValue,
        last_forgotten: JsValue,
    ) -> Result<JsValue, Error> {
        let result = self.get_updates_inner(last_position, last_forgotten)?;
        serde_wasm_bindgen::to_value(&result)
    }

    /// get SCT root
    /// SCT root can be compared with the root obtained by GRPC and verify that there is no divergence
    /// Returns: `Root`
    #[wasm_bindgen]
    pub fn get_nct_root(&mut self) -> Result<JsValue, Error> {
        let root = self.nct.root();
        serde_wasm_bindgen::to_value(&root)
    }

    /// get LP NFT asset
    /// Arguments:
    ///     position_value: `lp::position::Position`
    ///     position_state_value: `lp::position::State`
    /// Returns: `DenomMetadata`
    #[wasm_bindgen]
    pub fn get_lpnft_asset(
        &mut self,
        position_value: JsValue,
        position_state_value: JsValue,
    ) -> Result<JsValue, Error> {
        let position: Position = serde_wasm_bindgen::from_value(position_value)?;
        let position_state = serde_wasm_bindgen::from_value(position_state_value)?;
        let lp_nft = LpNft::new(position.id(), position_state);
        let denom = lp_nft.denom();
        serde_wasm_bindgen::to_value(&denom)
    }
}

impl ViewServer {
    pub fn get_updates_inner(
        &mut self,
        last_position: JsValue,
        last_forgotten: JsValue,
    ) -> WasmResult<ScanBlockResult> {
        let stored_position: Option<StoredPosition> =
            serde_wasm_bindgen::from_value(last_position)?;
        let stored_forgotten: Option<Forgotten> = serde_wasm_bindgen::from_value(last_forgotten)?;

        let nct_updates: Updates = self
            .nct
            .updates(
                stored_position.unwrap_or_default(),
                stored_forgotten.unwrap_or_default(),
            )
            .collect::<Updates>();

        let result = ScanBlockResult {
            height: self.latest_height,
            nct_updates,
            new_notes: self.notes.clone().into_values().collect(),
            new_swaps: self.swaps.clone().into_values().collect(),
        };
        Ok(result)
    }

    pub async fn scan_block_inner(
        &mut self,
        compact_block: JsValue,
    ) -> WasmResult<ScanBlockResult> {
        let block_proto: penumbra_proto::core::component::compact_block::v1alpha1::CompactBlock =
            serde_wasm_bindgen::from_value(compact_block)?;

        let block: CompactBlock = block_proto.try_into()?;

        // Newly detected spendable notes.
        let mut new_notes = Vec::new();
        // Newly detected claimable swaps.
        let mut new_swaps: Vec<SwapRecord> = Vec::new();

        for state_payload in block.state_payloads {
            let clone_payload = state_payload.clone();

            match state_payload {
                StatePayload::Note { note: payload, .. } => {
                    match payload.trial_decrypt(&self.fvk) {
                        Some(note) => {
                            let note_position = self.nct.insert(Keep, payload.note_commitment)?;

                            let source = clone_payload.source().cloned().unwrap_or_default();
                            let nullifier = Nullifier::derive(
                                self.fvk.nullifier_key(),
                                note_position,
                                clone_payload.commitment(),
                            );
                            let address_index = self
                                .fvk
                                .incoming()
                                .index_for_diversifier(note.diversifier());

                            let note_record = SpendableNoteRecord {
                                note_commitment: *clone_payload.commitment(),
                                height_spent: None,
                                height_created: block.height,
                                note: note.clone(),
                                address_index,
                                nullifier,
                                position: note_position,
                                source,
                            };
                            new_notes.push(note_record.clone());
                            self.notes
                                .insert(payload.note_commitment, note_record.clone());
                            self.notes_by_nullifier
                                .insert(nullifier, note_record.clone());
                        }
                        None => {
                            self.nct.insert(Forget, payload.note_commitment)?;
                        }
                    }
                }
                StatePayload::Swap { swap: payload, .. } => {
                    match payload.trial_decrypt(&self.fvk) {
                        Some(swap) => {
                            let swap_position = self.nct.insert(Keep, payload.commitment)?;
                            let batch_data =
                                block.swap_outputs.get(&swap.trading_pair).ok_or_else(|| {
                                    anyhow::anyhow!("server gave invalid compact block")
                                })?;

                            let source = clone_payload.source().cloned().unwrap_or_default();
                            let nullifier = Nullifier::derive(
                                self.fvk.nullifier_key(),
                                swap_position,
                                clone_payload.commitment(),
                            );

                            let swap_record = SwapRecord {
                                swap_commitment: *clone_payload.commitment(),
                                swap: swap.clone(),
                                position: swap_position,
                                nullifier,
                                source,
                                output_data: *batch_data,
                                height_claimed: None,
                            };
                            new_swaps.push(swap_record.clone());
                            self.swaps.insert(payload.commitment, swap_record);

                            let batch_data =
                                block.swap_outputs.get(&swap.trading_pair).ok_or_else(|| {
                                    anyhow::anyhow!("server gave invalid compact block")
                                })?;

                            let (output_1, output_2) = swap.output_notes(batch_data);

                            self.storage.store_advice(output_1).await?;
                            self.storage.store_advice(output_2).await?;
                        }
                        None => {
                            self.nct.insert(Forget, payload.commitment)?;
                        }
                    }
                }
                StatePayload::RolledUp(commitment) => {
                    if let std::collections::btree_map::Entry::Occupied(mut e) =
                        self.notes.entry(commitment)
                    {
                        // This is a note we anticipated, so retain its auth path.

                        let advice_result = self.storage.read_advice(commitment).await?;

                        match advice_result {
                            None => {}
                            Some(note) => {
                                let position = self.nct.insert(Keep, commitment)?;

                                let address_index_1 = self
                                    .fvk
                                    .incoming()
                                    .index_for_diversifier(note.diversifier());

                                let nullifier = Nullifier::derive(
                                    self.fvk.nullifier_key(),
                                    position,
                                    &commitment,
                                );

                                let source = clone_payload.source().cloned().unwrap_or_default();

                                let spendable_note = SpendableNoteRecord {
                                    note_commitment: note.commit(),
                                    height_spent: Some(u64::MAX),
                                    height_created: block.height,
                                    note: note.clone(),
                                    address_index: address_index_1,
                                    nullifier,
                                    position,
                                    source,
                                };

                                e.insert(spendable_note.clone());
                                new_notes.push(spendable_note.clone());
                            }
                        }
                    } else {
                        // This is someone else's note.
                        self.nct.insert(Forget, commitment)?;
                    }
                }
            }
        }

        self.nct.end_block()?;
        if block.epoch_root.is_some() {
            self.nct.end_epoch()?;
        }

        self.latest_height = block.height;

        let result = ScanBlockResult {
            height: self.latest_height,
            nct_updates: Default::default(),
            new_notes,
            new_swaps,
        };
        Ok(result)
    }
}

pub fn load_tree(stored_tree: StoredTree) -> Tree {
    let stored_position: StoredPosition = stored_tree.last_position.unwrap_or_default();
    let mut add_commitments = Tree::load(
        stored_position,
        stored_tree.last_forgotten.unwrap_or_default(),
    );

    for store_commitment in &stored_tree.commitments {
        add_commitments.insert(store_commitment.position, store_commitment.commitment)
    }
    let mut add_hashes = add_commitments.load_hashes();

    for stored_hash in &stored_tree.hashes {
        add_hashes.insert(stored_hash.position, stored_hash.height, stored_hash.hash);
    }
    add_hashes.finish()
}
