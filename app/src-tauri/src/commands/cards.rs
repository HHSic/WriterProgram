//! Setting cards (설정집): people, places, terms, and the kinds of card.

use std::path::Path;

use tauri::State;
use writer_core::cards::{self, Appearances, Card, CardSummary, CardType};
use writer_core::trash::{self, TrashItem};

use crate::error::{Res, fail};
use crate::state::AppState;

#[tauri::command]
pub async fn card_load(root: String, card_id: String) -> Res<Card> {
    cards::load(Path::new(&root), &card_id).map_err(fail)
}

#[tauri::command]
pub async fn card_create(
    state: State<'_, AppState>,
    root: String,
    type_id: String,
    name: String,
) -> Res<Card> {
    let _write = state.write();
    cards::create(Path::new(&root), &type_id, &name).map_err(fail)
}

#[tauri::command]
pub async fn card_save(state: State<'_, AppState>, root: String, card: Card) -> Res<CardSummary> {
    let _write = state.write();
    cards::save(Path::new(&root), &card).map_err(fail)
}

#[tauri::command]
pub async fn card_trash(
    state: State<'_, AppState>,
    root: String,
    card_id: String,
) -> Res<TrashItem> {
    let _write = state.write();
    trash::trash_card(Path::new(&root), &card_id).map_err(fail)
}

/// Chapters where a card's names appear.
#[tauri::command]
pub async fn card_appearances(root: String, card_id: String) -> Res<Appearances> {
    cards::appearances(Path::new(&root), &card_id).map_err(fail)
}

/// For each card, in how many chapters it appears.
#[tauri::command]
pub async fn card_counts(root: String) -> Res<Vec<(String, usize)>> {
    cards::appearance_counts(Path::new(&root)).map_err(fail)
}

#[tauri::command]
pub async fn card_type_add(
    state: State<'_, AppState>,
    root: String,
    name: String,
) -> Res<CardType> {
    let _write = state.write();
    cards::add_type(Path::new(&root), &name).map_err(fail)
}

#[tauri::command]
pub async fn card_type_update(state: State<'_, AppState>, root: String, kind: CardType) -> Res<()> {
    let _write = state.write();
    cards::update_type(Path::new(&root), &kind).map_err(fail)
}

#[tauri::command]
pub async fn card_type_remove(
    state: State<'_, AppState>,
    root: String,
    type_id: String,
) -> Res<()> {
    let _write = state.write();
    cards::remove_type(Path::new(&root), &type_id).map_err(fail)
}
