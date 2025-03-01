use penumbra_sct::Nullifier;
use tendermint::abci::{Event, EventAttributeIndexExt};

use crate::NotePayload;

pub fn spend(nullifier: &Nullifier) -> Event {
    Event::new(
        "action_spend",
        [("nullifier", nullifier.to_string()).index()],
    )
}

pub fn output(note_payload: &NotePayload) -> Event {
    Event::new(
        "action_output",
        [("note_commitment", note_payload.note_commitment.to_string()).index()],
    )
}
