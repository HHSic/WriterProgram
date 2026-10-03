// The app's state and everything that changes it, one file per area.
// Screens import from here (`../store`); the files import each other.

export type { Pane, SplitDir, Target } from '../lib/tabs';
export { useApp } from './state';
export type { HeldRemovals, Reading, SaveState, RightTab, FindScope, FindRequest, Jump, SettingsTab, Dialog, DocConflict, Toast, CornerNote, AiRequest } from './state';
export { findDoc, allManuscript, saveEverything, confirmClose, showToast, toastError, openDialog, closeDialog, openFind, jumpTo, setView, patchSummary } from './ui';
export type { DocPlace } from './ui';
export { registerEditor, isFocusedTab, openTarget, openDocInNewTab, activateTab, cycleTab, reorderTab, closeTab, closeActiveTab, reopenClosedTab, goBack, showInTab, focusPane, splitView, unsplit, toggleLock } from './tabs';
export { enterProject, loadCatalog, updateProject, applyFormat, openProject, recoverProject, leaveProject, refreshOverview } from './project';
export { selectDoc, mendDoc, addDoc, setStatus, setLocked, setTarget, renameDoc, moveDoc, trashDoc, addPart, renamePart, removePart } from './docs';
export { previewCard, openCard, refreshCardCounts, createCard, patchCardSummary, trashCard } from './cards';
export { loadNotes, focusNote, patchNote, addNote, addTextNote, showNote, trashNote, openNoteCounts } from './notes';
export { selectPart, newDocPartId, openNotesBoard, openTable } from './places';
export { markConflict, clearConflict, keepMine, takeTheirs, resolveCopy, moveProject } from './devices';
export { syncNow, linkProject, unlinkProject, answerRemovals, putOffRemovals } from './drives';
export { webPages, rememberWebPage, openWeb, clipPage } from './web';
export { loadJournal, setJournalEnabled, setAnchorAllowed, setAnchorWay, anchorNow, stampOnClosing, noteManuscriptOut, noteStatusChange } from './journal';
export { reviewDraft, keepReviewDraft, noteSent, openReview, takeBackCorrected, applyReview } from './exchange';
export type { WebPage } from './web';
export { setFocusMode, toggleFocusMode } from './focus';
export { startReading, pauseReading, resumeReading, stopReading, toggleReading, closeReadingNotice } from './reading';
export { loadAi, aiReady, updateAi, setAiKey, removeAiKey, askAi, putSynopsis } from './ai';
export { offerRescues, setRescueAside, takeRescue } from './rescue';
export { autoUpdateCheck, setAutoUpdateCheck, checkForUpdateDaily, checkForUpdateNow } from './update';
