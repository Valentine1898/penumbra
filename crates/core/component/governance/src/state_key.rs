use penumbra_sct::Nullifier;
use penumbra_stake::IdentityKey;

pub fn next_proposal_id() -> &'static str {
    "governance/next_proposal_id"
}

pub fn proposal_definition(proposal_id: u64) -> String {
    format!("governance/proposal/{proposal_id:020}/data")
}

pub fn dao_transaction(proposal_id: u64) -> String {
    format!("governance/proposal/{proposal_id:020}/dao_transaction")
}

pub fn proposal_state(proposal_id: u64) -> String {
    format!("governance/proposal/{proposal_id:020}/state")
}

pub fn proposal_deposit_amount(proposal_id: u64) -> String {
    format!("governance/proposal/{proposal_id:020}/deposit_amount")
}

pub fn proposal_voting_start(proposal_id: u64) -> String {
    format!("governance/proposal/{proposal_id:020}/voting_start")
}

pub fn proposal_voting_start_position(proposal_id: u64) -> String {
    format!("governance/proposal/{proposal_id:020}/voting_start_position")
}

pub fn proposal_voting_end(proposal_id: u64) -> String {
    format!("governance/proposal/{proposal_id:020}/voting_end")
}

pub fn unfinished_proposal(proposal_id: u64) -> String {
    format!("governance/unfinished_proposals/{proposal_id:020}")
}

pub fn all_unfinished_proposals() -> &'static str {
    // Note: this has to be the prefix of the `unfinished_proposal` function above.
    "governance/unfinished_proposals/"
}

pub fn voted_nullifier_lookup_for_proposal(proposal_id: u64, nullifier: &Nullifier) -> String {
    format!("governance/proposal/{proposal_id:020}/voted_nullifiers/{nullifier}")
}

pub fn rate_data_at_proposal_start(proposal_id: u64, identity_key: IdentityKey) -> String {
    format!("governance/proposal/{proposal_id:020}/rate_data_at_start/{identity_key}")
}

pub fn all_rate_data_at_proposal_start(proposal_id: u64) -> String {
    // Note: this has to be the prefix of the `rate_data_at_proposal_start` function above.
    format!("governance/proposal/{proposal_id:020}/rate_data_at_start/")
}

pub fn voting_power_at_proposal_start(proposal_id: u64, identity_key: IdentityKey) -> String {
    format!("governance/proposal/{proposal_id:020}/voting_power_at_start/{identity_key}")
}

pub fn all_voting_power_at_proposal_start(proposal_id: u64) -> String {
    // Note: this has to be the prefix of the `voting_power_at_proposal_start` function above.
    format!("governance/proposal/{proposal_id:020}/voting_power_at_start/")
}

pub fn validator_vote(proposal_id: u64, identity_key: IdentityKey) -> String {
    format!("governance/validator_vote/{proposal_id:020}/{identity_key}")
}

pub fn all_validator_votes_for_proposal(proposal_id: u64) -> String {
    // Note: this has to be the prefix of the `validator_vote` function above.
    format!("governance/validator_vote/{proposal_id:020}/")
}

pub fn tallied_delegator_votes(proposal_id: u64, identity_key: IdentityKey) -> String {
    format!("governance/tallied_delegator_votes/{proposal_id:020}/{identity_key}")
}

pub fn all_tallied_delegator_votes_for_proposal(proposal_id: u64) -> String {
    // Note: this has to be the prefix of the `tallied_delegator_votes` function above.
    format!("governance/tallied_delegator_votes/{proposal_id:020}/")
}

pub fn untallied_delegator_vote(
    proposal_id: u64,
    identity_key: IdentityKey,
    nullifier: &Nullifier,
) -> String {
    format!("governance/untallied_delegator_vote/{proposal_id:020}/{identity_key}/{nullifier}")
}

pub fn all_untallied_delegator_votes_for_proposal(proposal_id: u64) -> String {
    // Note: this has to be the prefix of the `untallied_delegator_vote` function above.
    format!("governance/untallied_delegator_vote/{proposal_id:020}/")
}

pub fn all_untallied_delegator_votes() -> &'static str {
    // Note: this has to be the prefix of the `untallied_delegator_vote` function above.
    "governance/untallied_delegator_vote/"
}

pub fn deliver_single_dao_transaction_at_height(block_height: u64, proposal_id: u64) -> String {
    format!("governance/deliver_dao_transactions/{block_height}/{proposal_id:020}")
}

pub fn deliver_dao_transactions_at_height(block_height: u64) -> String {
    // Note: this has to be the prefix of the `deliver_single_dao_transaction_at_height` function above.
    format!("governance/deliver_dao_transactions/{block_height}/")
}

pub fn change_chain_params_at_height(block_height: u64) -> String {
    format!("governance/change_chain_params/{block_height}/")
}

// Used for object store:

pub fn proposal_started() -> &'static str {
    "governance/proposal_started"
}
