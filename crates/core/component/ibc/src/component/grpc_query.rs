use async_trait::async_trait;
use ibc_proto::ibc::core::channel::v1::query_server::Query as ConsensusQuery;
use ibc_proto::ibc::core::channel::v1::{
    PacketState, QueryChannelClientStateRequest, QueryChannelClientStateResponse,
    QueryChannelConsensusStateRequest, QueryChannelConsensusStateResponse, QueryChannelRequest,
    QueryChannelResponse, QueryChannelsRequest, QueryChannelsResponse,
    QueryConnectionChannelsRequest, QueryConnectionChannelsResponse,
    QueryNextSequenceReceiveRequest, QueryNextSequenceReceiveResponse,
    QueryPacketAcknowledgementRequest, QueryPacketAcknowledgementResponse,
    QueryPacketAcknowledgementsRequest, QueryPacketAcknowledgementsResponse,
    QueryPacketCommitmentRequest, QueryPacketCommitmentResponse, QueryPacketCommitmentsRequest,
    QueryPacketCommitmentsResponse, QueryPacketReceiptRequest, QueryPacketReceiptResponse,
    QueryUnreceivedAcksRequest, QueryUnreceivedAcksResponse, QueryUnreceivedPacketsRequest,
    QueryUnreceivedPacketsResponse,
};
use ibc_proto::ibc::core::client::v1::query_server::Query as ClientQuery;
use ibc_proto::ibc::core::client::v1::{
    Height, IdentifiedClientState, QueryClientParamsRequest, QueryClientParamsResponse,
    QueryClientStateRequest, QueryClientStateResponse, QueryClientStatesRequest,
    QueryClientStatesResponse, QueryClientStatusRequest, QueryClientStatusResponse,
    QueryConsensusStateHeightsRequest, QueryConsensusStateHeightsResponse,
    QueryConsensusStateRequest, QueryConsensusStateResponse, QueryConsensusStatesRequest,
    QueryConsensusStatesResponse, QueryUpgradedClientStateRequest,
    QueryUpgradedClientStateResponse, QueryUpgradedConsensusStateRequest,
    QueryUpgradedConsensusStateResponse,
};
use ibc_proto::ibc::core::connection::v1::query_server::Query as ConnectionQuery;
use ibc_proto::ibc::core::connection::v1::{
    ConnectionEnd, QueryClientConnectionsRequest, QueryClientConnectionsResponse,
    QueryConnectionClientStateRequest, QueryConnectionClientStateResponse,
    QueryConnectionConsensusStateRequest, QueryConnectionConsensusStateResponse,
    QueryConnectionRequest, QueryConnectionResponse, QueryConnectionsRequest,
    QueryConnectionsResponse,
};
use ibc_types::core::channel::{ChannelId, IdentifiedChannelEnd, PortId};
use ibc_types::core::client::ClientId;
use ibc_types::core::connection::{ConnectionId, IdentifiedConnectionEnd};
use ibc_types::DomainType;
use penumbra_chain::component::AppHashRead;
use prost::Message;
use std::str::FromStr;
use tonic::{Response, Status};

use crate::component::{ClientStateReadExt, ConnectionStateReadExt};

use super::{state_key, ChannelStateReadExt};

#[derive(Clone)]
pub struct IbcQuery(penumbra_storage::Storage);

impl IbcQuery {
    pub fn new(storage: penumbra_storage::Storage) -> Self {
        Self(storage)
    }
}

#[async_trait]
impl ConnectionQuery for IbcQuery {
    /// Connection queries an IBC connection end.
    async fn connection(
        &self,
        request: tonic::Request<QueryConnectionRequest>,
    ) -> std::result::Result<tonic::Response<QueryConnectionResponse>, tonic::Status> {
        let snapshot = self.0.latest_snapshot();
        let connection_id = &ConnectionId::from_str(&request.get_ref().connection_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid connection id: {e}")))?;

        let (conn, proof) = snapshot
            .get_with_proof_to_apphash(
                state_key::connections::by_connection_id(connection_id)
                    .as_bytes()
                    .to_vec(),
            )
            .await
            .map(|res| {
                let conn = res
                    .0
                    .map(|conn_bytes| ConnectionEnd::decode(conn_bytes.as_ref()))
                    .transpose();

                (conn, res.1)
            })
            .map_err(|e| tonic::Status::aborted(format!("couldn't get connection: {e}")))?;

        let conn =
            conn.map_err(|e| tonic::Status::aborted(format!("couldn't decode connection: {e}")))?;

        let height = Height {
            revision_number: 0,
            revision_height: snapshot.version().into(),
        };

        let res = QueryConnectionResponse {
            connection: conn,
            proof: proof.encode_to_vec(),
            proof_height: Some(height),
        };

        Ok(tonic::Response::new(res))
    }

    async fn connection_params(
        &self,
        _request: tonic::Request<
            ibc_proto::ibc::core::connection::v1::QueryConnectionParamsRequest,
        >,
    ) -> std::result::Result<
        tonic::Response<ibc_proto::ibc::core::connection::v1::QueryConnectionParamsResponse>,
        tonic::Status,
    > {
        Err(tonic::Status::unimplemented("not implemented"))
    }

    /// Connections queries all the IBC connections of a chain.
    async fn connections(
        &self,
        _request: tonic::Request<QueryConnectionsRequest>,
    ) -> std::result::Result<tonic::Response<QueryConnectionsResponse>, tonic::Status> {
        let snapshot = self.0.latest_snapshot();
        let height = snapshot.version();

        let connection_counter = snapshot
            .get_connection_counter()
            .await
            .map_err(|e| tonic::Status::aborted(format!("couldn't get connection counter: {e}")))?;

        let mut connections = vec![];
        for conn_idx in 0..connection_counter.0 {
            let conn_id = ConnectionId(format!("connection-{}", conn_idx));
            let connection = snapshot
                .get_connection(&conn_id)
                .await
                .map_err(|e| {
                    tonic::Status::aborted(format!("couldn't get connection {conn_id}: {e}"))
                })?
                .ok_or("unable to get connection")
                .map_err(|e| {
                    tonic::Status::aborted(format!("couldn't get connection {conn_id}: {e}"))
                })?;
            let id_conn = IdentifiedConnectionEnd {
                connection_id: conn_id,
                connection_end: connection,
            };
            connections.push(id_conn.into());
        }

        let height = Height {
            revision_number: 0,
            revision_height: height.into(),
        };

        let res = QueryConnectionsResponse {
            connections,
            pagination: None,
            height: Some(height),
        };

        Ok(tonic::Response::new(res))
    }
    /// ClientConnections queries the connection paths associated with a client
    /// state.
    async fn client_connections(
        &self,
        _request: tonic::Request<QueryClientConnectionsRequest>,
    ) -> std::result::Result<tonic::Response<QueryClientConnectionsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// ConnectionClientState queries the client state associated with the
    /// connection.
    async fn connection_client_state(
        &self,
        _request: tonic::Request<QueryConnectionClientStateRequest>,
    ) -> std::result::Result<tonic::Response<QueryConnectionClientStateResponse>, tonic::Status>
    {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// ConnectionConsensusState queries the consensus state associated with the
    /// connection.
    async fn connection_consensus_state(
        &self,
        _request: tonic::Request<QueryConnectionConsensusStateRequest>,
    ) -> std::result::Result<tonic::Response<QueryConnectionConsensusStateResponse>, tonic::Status>
    {
        Err(tonic::Status::unimplemented("not implemented"))
    }
}

#[async_trait]
impl ConsensusQuery for IbcQuery {
    /// Channel queries an IBC Channel.
    async fn channel(
        &self,
        _request: tonic::Request<QueryChannelRequest>,
    ) -> std::result::Result<tonic::Response<QueryChannelResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// Channels queries all the IBC channels of a chain.
    async fn channels(
        &self,
        _request: tonic::Request<QueryChannelsRequest>,
    ) -> std::result::Result<tonic::Response<QueryChannelsResponse>, tonic::Status> {
        let snapshot = self.0.latest_snapshot();
        let height = Height {
            revision_number: 0,
            revision_height: snapshot.version().into(),
        };

        let channel_counter = snapshot
            .get_channel_counter()
            .await
            .map_err(|e| tonic::Status::aborted(format!("couldn't get channel counter: {e}")))?;

        let mut channels = vec![];
        for chan_idx in 0..channel_counter {
            let chan_id = ChannelId(format!("channel-{}", chan_idx));
            let channel = snapshot
                .get_channel(&chan_id, &PortId::transfer())
                .await
                .map_err(|e| {
                    tonic::Status::aborted(format!("couldn't get channel {chan_id}: {e}"))
                })?
                .ok_or("unable to get channel")
                .map_err(|e| {
                    tonic::Status::aborted(format!("couldn't get channel {chan_id}: {e}"))
                })?;

            let id_chan = IdentifiedChannelEnd {
                channel_id: chan_id,
                port_id: PortId::transfer(),
                channel_end: channel,
            };
            channels.push(id_chan.into());
        }

        let res = QueryChannelsResponse {
            channels,
            pagination: None,
            height: Some(height),
        };

        Ok(tonic::Response::new(res))
    }
    /// ConnectionChannels queries all the channels associated with a connection
    /// end.
    async fn connection_channels(
        &self,
        request: tonic::Request<QueryConnectionChannelsRequest>,
    ) -> std::result::Result<tonic::Response<QueryConnectionChannelsResponse>, tonic::Status> {
        let snapshot = self.0.latest_snapshot();
        let height = Height {
            revision_number: 0,
            revision_height: snapshot.version().into(),
        };
        let request = request.get_ref();

        let connection_id: ConnectionId = ConnectionId::from_str(&request.connection)
            .map_err(|e| tonic::Status::aborted(format!("invalid connection id: {e}")))?;

        // look up all of the channels for this connection
        let channel_counter = snapshot
            .get_channel_counter()
            .await
            .map_err(|e| tonic::Status::aborted(format!("couldn't get channel counter: {e}")))?;

        let mut channels = vec![];
        for chan_idx in 0..channel_counter {
            let chan_id = ChannelId(format!("channel-{}", chan_idx));
            let channel = snapshot
                .get_channel(&chan_id, &PortId::transfer())
                .await
                .map_err(|e| {
                    tonic::Status::aborted(format!("couldn't get channel {chan_id}: {e}"))
                })?
                .ok_or("unable to get channel")
                .map_err(|e| {
                    tonic::Status::aborted(format!("couldn't get channel {chan_id}: {e}"))
                })?;
            if channel.connection_hops.contains(&connection_id) {
                let id_chan = IdentifiedChannelEnd {
                    channel_id: chan_id,
                    port_id: PortId::transfer(),
                    channel_end: channel,
                };
                channels.push(id_chan.into());
            }
        }

        let res = QueryConnectionChannelsResponse {
            channels,
            pagination: None,
            height: Some(height),
        };

        Ok(tonic::Response::new(res))
    }
    /// ChannelClientState queries for the client state for the channel associated
    /// with the provided channel identifiers.
    async fn channel_client_state(
        &self,
        _request: tonic::Request<QueryChannelClientStateRequest>,
    ) -> std::result::Result<tonic::Response<QueryChannelClientStateResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// ChannelConsensusState queries for the consensus state for the channel
    /// associated with the provided channel identifiers.
    async fn channel_consensus_state(
        &self,
        _request: tonic::Request<QueryChannelConsensusStateRequest>,
    ) -> std::result::Result<tonic::Response<QueryChannelConsensusStateResponse>, tonic::Status>
    {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// PacketCommitment queries a stored packet commitment hash.
    async fn packet_commitment(
        &self,
        _request: tonic::Request<QueryPacketCommitmentRequest>,
    ) -> std::result::Result<tonic::Response<QueryPacketCommitmentResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// PacketCommitments returns all the packet commitments hashes associated
    /// with a channel.
    async fn packet_commitments(
        &self,
        request: tonic::Request<QueryPacketCommitmentsRequest>,
    ) -> std::result::Result<tonic::Response<QueryPacketCommitmentsResponse>, tonic::Status> {
        let snapshot = self.0.latest_snapshot();
        let height = snapshot.version();
        let request = request.get_ref();

        let chan_id: ChannelId = ChannelId::from_str(&request.channel_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid channel id: {e}")))?;
        let port_id: PortId = PortId::from_str(&request.port_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid port id: {e}")))?;

        let mut commitment_states = vec![];
        let commitment_counter = snapshot
            .get_send_sequence(&chan_id, &port_id)
            .await
            .map_err(|e| {
                tonic::Status::aborted(format!(
                    "couldn't get send sequence for channel {chan_id} and port {port_id}: {e}"
                ))
            })?;

        // this starts at 1; the first commitment index is 1 (from ibc spec)
        for commitment_idx in 1..commitment_counter {
            let commitment = snapshot
                .get_packet_commitment_by_id(&chan_id, &port_id, commitment_idx)
                .await.map_err(|e| {
                    tonic::Status::aborted(format!(
                        "couldn't get packet commitment for channel {chan_id} and port {port_id} at index {commitment_idx}: {e}"
                    ))
                })?;
            if commitment.is_none() {
                continue;
            }
            let commitment = commitment.expect("commitment existence was checked earlier");

            let commitment_state = PacketState {
                port_id: request.port_id.clone(),
                channel_id: request.channel_id.clone(),
                sequence: commitment_idx,
                data: commitment.clone(),
            };

            commitment_states.push(commitment_state);
        }

        let height = Height {
            revision_number: 0,
            revision_height: height.into(),
        };

        let res = QueryPacketCommitmentsResponse {
            commitments: commitment_states,
            pagination: None,
            height: Some(height),
        };

        Ok(tonic::Response::new(res))
    }
    /// PacketReceipt queries if a given packet sequence has been received on the
    /// queried chain
    async fn packet_receipt(
        &self,
        _request: tonic::Request<QueryPacketReceiptRequest>,
    ) -> std::result::Result<tonic::Response<QueryPacketReceiptResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// PacketAcknowledgement queries a stored packet acknowledgement hash.
    async fn packet_acknowledgement(
        &self,
        _request: tonic::Request<QueryPacketAcknowledgementRequest>,
    ) -> std::result::Result<tonic::Response<QueryPacketAcknowledgementResponse>, tonic::Status>
    {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// PacketAcknowledgements returns all the packet acknowledgements associated
    /// with a channel.
    async fn packet_acknowledgements(
        &self,
        request: tonic::Request<QueryPacketAcknowledgementsRequest>,
    ) -> std::result::Result<tonic::Response<QueryPacketAcknowledgementsResponse>, tonic::Status>
    {
        let snapshot = self.0.latest_snapshot();
        let height = Height {
            revision_number: 0,
            revision_height: snapshot.version().into(),
        };
        let request = request.get_ref();

        let chan_id: ChannelId = ChannelId::from_str(&request.channel_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid channel id: {e}")))?;
        let port_id: PortId = PortId::from_str(&request.port_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid port id: {e}")))?;

        let ack_counter = snapshot
            .get_ack_sequence(&chan_id, &port_id)
            .await
            .map_err(|e| {
                tonic::Status::aborted(format!(
                    "couldn't get ack sequence for channel {chan_id} and port {port_id}: {e}"
                ))
            })?;

        let mut acks = vec![];
        for ack_idx in 0..ack_counter {
            let maybe_ack = snapshot
                .get_packet_acknowledgement(&port_id, &chan_id, ack_idx)
                .await.map_err(|e| {
                    tonic::Status::aborted(format!(
                        "couldn't get packet acknowledgement for channel {chan_id} and port {port_id} at index {ack_idx}: {e}"
                    ))
                })?;

            // Only include the ack if it was found; otherwise, signal lack of
            // by omitting it from the response.
            if let Some(ack) = maybe_ack {
                let ack_state = PacketState {
                    port_id: request.port_id.clone(),
                    channel_id: request.channel_id.clone(),
                    sequence: ack_idx,
                    data: ack.clone(),
                };

                acks.push(ack_state);
            }
        }

        let res = QueryPacketAcknowledgementsResponse {
            acknowledgements: acks,
            pagination: None,
            height: Some(height),
        };

        Ok(tonic::Response::new(res))
    }
    /// UnreceivedPackets returns all the unreceived IBC packets associated with a
    /// channel and sequences.
    async fn unreceived_packets(
        &self,
        request: tonic::Request<QueryUnreceivedPacketsRequest>,
    ) -> std::result::Result<tonic::Response<QueryUnreceivedPacketsResponse>, tonic::Status> {
        let snapshot = self.0.latest_snapshot();
        let height = snapshot.version();
        let request = request.get_ref();

        let chan_id: ChannelId = ChannelId::from_str(&request.channel_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid channel id: {e}")))?;
        let port_id: PortId = PortId::from_str(&request.port_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid port id: {e}")))?;

        let mut unreceived_seqs = vec![];

        for seq in request.packet_commitment_sequences.clone() {
            if seq == 0 {
                return Err(tonic::Status::aborted(format!(
                    "packet sequence {} cannot be 0",
                    seq
                )));
            }

            if !snapshot
                .seen_packet_by_channel(&chan_id, &port_id, seq)
                .await.map_err(|e| {
                    tonic::Status::aborted(format!(
                        "couldn't get packet commitment for channel {chan_id} and port {port_id} at index {seq}: {e}"
                    ))
                })?
            {
                unreceived_seqs.push(seq);
            }
        }

        let height = Height {
            revision_number: 0,
            revision_height: height.into(),
        };

        let res = QueryUnreceivedPacketsResponse {
            sequences: unreceived_seqs,
            height: Some(height),
        };

        Ok(tonic::Response::new(res))
    }
    /// UnreceivedAcks returns all the unreceived IBC acknowledgements associated
    /// with a channel and sequences.
    async fn unreceived_acks(
        &self,
        request: tonic::Request<QueryUnreceivedAcksRequest>,
    ) -> std::result::Result<tonic::Response<QueryUnreceivedAcksResponse>, tonic::Status> {
        let snapshot = self.0.latest_snapshot();
        let height = Height {
            revision_number: 0,
            revision_height: snapshot.version().into(),
        };
        let request = request.get_ref();

        let chan_id: ChannelId = ChannelId::from_str(&request.channel_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid channel id: {e}")))?;
        let port_id: PortId = PortId::from_str(&request.port_id)
            .map_err(|e| tonic::Status::aborted(format!("invalid port id: {e}")))?;

        let mut unreceived_seqs = vec![];

        for seq in request.packet_ack_sequences.clone() {
            if seq == 0 {
                return Err(tonic::Status::aborted(format!(
                    "packet sequence {} cannot be 0",
                    seq
                )));
            }

            if snapshot
                .get_packet_commitment_by_id(&chan_id, &port_id, seq)
                .await.map_err(|e| {
                    tonic::Status::aborted(format!(
                        "couldn't get packet commitment for channel {chan_id} and port {port_id} at index {seq}: {e}"
                    ))
                })?
                .is_some()
            {
                unreceived_seqs.push(seq);
            }
        }

        let res = QueryUnreceivedAcksResponse {
            sequences: unreceived_seqs,
            height: Some(height),
        };

        Ok(tonic::Response::new(res))
    }

    /// NextSequenceReceive returns the next receive sequence for a given channel.
    async fn next_sequence_receive(
        &self,
        _request: tonic::Request<QueryNextSequenceReceiveRequest>,
    ) -> std::result::Result<tonic::Response<QueryNextSequenceReceiveResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
}

#[async_trait]
impl ClientQuery for IbcQuery {
    async fn client_state(
        &self,
        _request: tonic::Request<QueryClientStateRequest>,
    ) -> std::result::Result<Response<QueryClientStateResponse>, Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// ClientStates queries all the IBC light clients of a chain.
    async fn client_states(
        &self,
        _request: tonic::Request<QueryClientStatesRequest>,
    ) -> std::result::Result<tonic::Response<QueryClientStatesResponse>, tonic::Status> {
        let snapshot = self.0.latest_snapshot();

        let client_counter = snapshot
            .client_counter()
            .await
            .map_err(|e| tonic::Status::aborted(format!("couldn't get client counter: {e}")))?
            .0;

        let mut client_states = vec![];
        for client_idx in 0..client_counter {
            // NOTE: currently, we only look up tendermint clients, because we only support tendermint clients.
            let client_id = ClientId::from_str(format!("07-tendermint-{}", client_idx).as_str())
                .map_err(|e| tonic::Status::aborted(format!("invalid client id: {e}")))?;
            let client_state = snapshot.get_client_state(&client_id).await;
            let id_client = IdentifiedClientState {
                client_id: client_id.to_string(),
                client_state: client_state.ok().map(|state| state.into()), // send None if we couldn't find the client state
            };
            client_states.push(id_client);
        }

        let res = QueryClientStatesResponse {
            client_states,
            pagination: None,
        };

        Ok(tonic::Response::new(res))
    }
    /// ConsensusState queries a consensus state associated with a client state at
    /// a given height.
    async fn consensus_state(
        &self,
        _request: tonic::Request<QueryConsensusStateRequest>,
    ) -> std::result::Result<tonic::Response<QueryConsensusStateResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// ConsensusStates queries all the consensus state associated with a given
    /// client.
    async fn consensus_states(
        &self,
        _request: tonic::Request<QueryConsensusStatesRequest>,
    ) -> std::result::Result<tonic::Response<QueryConsensusStatesResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// ConsensusStateHeights queries the height of every consensus states associated with a given client.
    async fn consensus_state_heights(
        &self,
        _request: tonic::Request<QueryConsensusStateHeightsRequest>,
    ) -> std::result::Result<tonic::Response<QueryConsensusStateHeightsResponse>, tonic::Status>
    {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// Status queries the status of an IBC client.
    async fn client_status(
        &self,
        _request: tonic::Request<QueryClientStatusRequest>,
    ) -> std::result::Result<tonic::Response<QueryClientStatusResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// ClientParams queries all parameters of the ibc client.
    async fn client_params(
        &self,
        _request: tonic::Request<QueryClientParamsRequest>,
    ) -> std::result::Result<tonic::Response<QueryClientParamsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// UpgradedClientState queries an Upgraded IBC light client.
    async fn upgraded_client_state(
        &self,
        _request: tonic::Request<QueryUpgradedClientStateRequest>,
    ) -> std::result::Result<tonic::Response<QueryUpgradedClientStateResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("not implemented"))
    }
    /// UpgradedConsensusState queries an Upgraded IBC consensus state.
    async fn upgraded_consensus_state(
        &self,
        _request: tonic::Request<QueryUpgradedConsensusStateRequest>,
    ) -> std::result::Result<tonic::Response<QueryUpgradedConsensusStateResponse>, tonic::Status>
    {
        Err(tonic::Status::unimplemented("not implemented"))
    }
}
