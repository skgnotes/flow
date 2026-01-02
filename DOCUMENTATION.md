# Journal App - Complete Technical Documentation

## Project Overview

A privacy-first, fully local journaling application for macOS built with Tauri 2, React, and TypeScript. All journal entries are stored as plain markdown files with YAML frontmatter in the local file system.

**Version:** 1.1
**Platform:** macOS
**Tech Stack:** Tauri 2, React, TypeScript, Rust, Whisper (local)
**Data Storage:** Plain markdown files in `~/Documents/Project Data Files/Journal/`
**Whisper Model:** `~/Documents/Project Data Files/Journal/models/ggml-base.en.bin`

## Screenshot

![Journal App Screenshot](screenshot.png)

*The Journal App interface showing the sidebar with entries and the blog-style editor view.*

## Core Design Principles

1. **Privacy First:** All data stored locally, no cloud sync, no telemetry
2. **Future-Proof Storage:** Plain markdown files that can be read by any text editor
3. **Simplicity:** Clean, focused feature set without unnecessary complexity
4. **Blog-Style Interface:** Distraction-free writing experience with clean, centered layout
5. **Auto-Save:** Changes automatically saved with 1-second debounce
6. **Voice-First Optional:** Local Whisper transcription for voice memos and dictation

## Architecture

```
journal-app/
├── src/                          # React frontend
│   ├── App.tsx                   # Main application component
│   ├── App.css                   # Application styles
│   ├── main.tsx                  # React entry point
│   └── hooks/                    # Custom React hooks
│       ├── useWhisperModel.ts    # Whisper model download/status
│       ├── useVoiceRecording.ts  # Push-to-talk recording
│       └── useAudioImport.ts     # Audio file import
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── lib.rs                # Tauri commands and file operations
│   │   ├── whisper_model.rs      # Whisper model management
│   │   ├── audio_recorder.rs     # Microphone recording
│   │   ├── audio_import.rs       # Audio format conversion
│   │   └── transcription.rs      # Whisper transcription
│   ├── Cargo.toml                # Rust dependencies
│   └── tauri.conf.json           # Tauri configuration
└── package.json                  # Node.js dependencies
```

## File Format Specification

### Journal Entry Structure

Each journal entry is a markdown file with YAML frontmatter:

```markdown
---
title: My Journal Entry
date: January 1, 2026
---

This is the content of my journal entry.
It can contain any markdown formatting.
```

### Filename Convention

- **With Title:** `{title}.md` (e.g., `My Journal Entry.md`)
- **Without Title:** `{date}.md` (e.g., `January 1, 2026.md`)

### Bidirectional Sync

- When title is edited → filename updates to match title
- When filename is renamed → title field updates to match filename
- Date field is always editable but doesn't affect filename if title exists

## Backend Implementation (Rust)

### File Location: `src-tauri/src/lib.rs`

### Dependencies (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-fs = "2"
tauri-plugin-dialog = "2"
tauri-plugin-global-shortcut = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
dirs = "5.0"
regex = "1"

# Audio recording
cpal = "0.15"
hound = "3.5"

# Whisper transcription
whisper-rs = "0.12"

# Audio format conversion
symphonia = { version = "0.5", features = ["mp3", "aac", "isomp4"] }

# Async & utilities
tokio = { version = "1", features = ["sync"] }
once_cell = "1.19"
reqwest = { version = "0.11", features = ["stream"] }
futures-util = "0.3"
```

### Data Structures

```rust
#[derive(Serialize, Deserialize)]
struct EntryInfo {
    filename: String,
    title: String,
    date: String,
}
```

### Core Functions

#### 1. `get_journal_dir() -> PathBuf`
Returns the journal directory path: `~/Documents/Project Data Files/Journal/`

```rust
fn get_journal_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join("Documents").join("Project Data Files").join("Journal")
}
```

#### 2. `parse_frontmatter(content: &str) -> (String, String)`
Extracts title and date from YAML frontmatter using regex.

**Regex Patterns:**
- Frontmatter: `(?s)^---\n(.*?)\n---`
- Title: `(?m)^title:\s*(.*)$`
- Date: `(?m)^date:\s*(.*)$`

**Returns:** `(title, date)` tuple

#### 3. Tauri Commands

##### `list_entries() -> Result<Vec<EntryInfo>, String>`
- Scans journal directory for `.md` files
- Parses frontmatter from each file
- Sorts by date (newest first) using chrono::NaiveDate
- Fallback: sorts by filename if date parsing fails
- Creates directory if it doesn't exist

**Date Parsing Formats:**
- `%B %-d, %Y` (e.g., "January 1, 2026")
- `%B %d, %Y` (e.g., "January 01, 2026")

##### `read_entry(filename: String) -> Result<String, String>`
- Reads complete file content including frontmatter
- Returns raw markdown text

##### `save_entry(filename: String, content: String) -> Result<(), String>`
- Writes content to file
- Creates directory if needed

##### `create_entry() -> Result<String, String>`
- Generates filename from current date
- Creates file with initial frontmatter template
- Auto-populates date field
- Title field starts empty

**Template:**
```markdown
---
title:
date: {current_date}
---

```

##### `update_entry_metadata(filename: String, title: String, date: String, content: String) -> Result<String, String>`
- Updates file content with new frontmatter
- Determines new filename based on title/date
- Renames file if needed
- Prevents overwriting existing files
- Returns new filename

**Filename Logic:**
```rust
let new_filename = if title.trim().is_empty() {
    format!("{}.md", date)
} else {
    format!("{}.md", title.trim())
};
```

##### `rename_entry(old_filename: String, new_filename: String) -> Result<(), String>`
- Renames file in journal directory
- Ensures `.md` extension
- Validates file existence
- Prevents name conflicts

##### `delete_entry(filename: String) -> Result<(), String>`
- Deletes specified journal entry
- Validates file exists before deletion
- Returns error if file not found

### Command Registration

```rust
.invoke_handler(tauri::generate_handler![
    // Journal commands
    list_entries,
    read_entry,
    save_entry,
    create_entry,
    rename_entry,
    update_entry_metadata,
    delete_entry,
    // Voice commands
    whisper_model::check_whisper_model,
    whisper_model::download_whisper_model,
    start_recording,
    stop_recording_and_transcribe,
    transcribe_audio_file
])
```

## Voice Features Implementation (Rust)

### Whisper Model Management (`whisper_model.rs`)

**Model Storage:** `~/Documents/Project Data Files/Journal/models/ggml-base.en.bin`
**Model Size:** ~142MB (base.en - English only, good accuracy/speed balance)
**Model URL:** `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin`

#### Commands

##### `check_whisper_model() -> Result<bool, String>`
- Checks if model file exists and is valid (>100MB)
- Returns true if model is ready for use

##### `download_whisper_model(window: Window) -> Result<(), String>`
- Downloads model from HuggingFace with progress tracking
- Emits `whisper-download-progress` events (0-100)
- Creates models directory if needed
- Verifies download integrity

### Audio Recording (`audio_recorder.rs`)

Uses thread-safe architecture to work with Tauri's state management.

#### Key Components

```rust
pub struct SharedSamples {
    samples: Mutex<Vec<f32>>,
    is_recording: AtomicBool,
}
```

##### `start_recording_thread(shared: Arc<SharedSamples>) -> Result<JoinHandle<()>, String>`
- Spawns background thread for recording
- Uses `cpal` for cross-platform microphone access
- Records at 16kHz mono (Whisper's required format)
- Automatically converts stereo to mono
- Resamples if device doesn't support 16kHz

### Audio Import (`audio_import.rs`)

##### `convert_to_whisper_format(path: &Path) -> Result<Vec<f32>, String>`
- Converts MP3, M4A, WAV, OGG, FLAC, AAC to Whisper format
- Uses `symphonia` for decoding
- Converts to 16kHz mono f32 samples
- Handles stereo-to-mono conversion
- Resamples using linear interpolation

### Transcription (`transcription.rs`)

Uses lazy-loaded global Whisper context for efficiency.

```rust
static WHISPER_CTX: Lazy<Mutex<Option<WhisperContext>>> = Lazy::new(|| Mutex::new(None));
```

##### `transcribe_audio(samples: &[f32]) -> Result<String, String>`
- Transcribes 16kHz mono f32 samples
- Uses greedy sampling strategy
- English-only for speed
- Returns concatenated segment text

### Voice Tauri Commands (lib.rs)

##### `start_recording(state: State<RecorderState>) -> Result<(), String>`
- Starts microphone recording in background thread
- Returns error if already recording

##### `stop_recording_and_transcribe(state: State<RecorderState>) -> Result<String, String>`
- Stops recording and waits for thread to finish
- Transcribes recorded audio
- Returns transcribed text

##### `transcribe_audio_file(path: String) -> Result<String, String>`
- Converts audio file to Whisper format
- Transcribes and returns text

## Frontend Implementation (React + TypeScript)

### File Location: `src/App.tsx`

### Interfaces

```typescript
interface EntryMetadata {
  title: string;
  date: string;
  content: string;
}

interface EntryInfo {
  filename: string;
  title: string;
  date: string;
}
```

### State Management

```typescript
const [entries, setEntries] = useState<EntryInfo[]>([]);
const [selectedEntry, setSelectedEntry] = useState<string | null>(null);
const [title, setTitle] = useState<string>("");
const [date, setDate] = useState<string>("");
const [content, setContent] = useState<string>("");
const [isSaving, setIsSaving] = useState(false);
const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null); // Filename pending deletion
```

### Core Functions

#### `parseFrontmatter(text: string): EntryMetadata`
Client-side frontmatter parser that extracts title, date, and content.

**Regex:** `/^---\n([\s\S]*?)\n---\n([\s\S]*)$/`

**Returns:**
- `title`: Extracted from frontmatter or empty string
- `date`: Extracted from frontmatter or empty string
- `content`: Everything after frontmatter block

#### `loadEntries()`
- Calls `list_entries` Tauri command
- Updates `entries` state
- Handles errors silently (logs to console)

#### `loadEntry(filename: string)`
- Calls `read_entry` Tauri command
- Parses frontmatter
- Updates title, date, content states
- Sets selected entry

#### `saveEntry()`
- Debounced auto-save function (1000ms delay)
- Calls `update_entry_metadata` Tauri command
- Handles filename changes
- Reloads entry list if filename changed
- Shows saving indicator

**Implementation:**
```typescript
const saveEntry = useCallback(async () => {
  if (!selectedEntry) return;

  setIsSaving(true);
  try {
    const newFilename = await invoke<string>("update_entry_metadata", {
      filename: selectedEntry,
      title: title,
      date: date,
      content: content,
    });

    if (newFilename !== selectedEntry) {
      setSelectedEntry(newFilename);
      await loadEntries();
    }
  } catch (error) {
    console.error("Failed to save entry:", error);
  } finally {
    setIsSaving(false);
  }
}, [selectedEntry, title, date, content]);
```

#### Auto-Save Effect

```typescript
useEffect(() => {
  if (!selectedEntry) return;

  const timeout = setTimeout(() => {
    saveEntry();
  }, 1000);

  return () => clearTimeout(timeout);
}, [title, date, content, selectedEntry, saveEntry]);
```

#### `createNewEntry()`
- Calls `create_entry` Tauri command
- Reloads entry list
- Automatically opens new entry

#### Delete Functions

The delete functionality uses a custom React modal instead of the native `confirm()` dialog, as browser dialogs don't work in Tauri's webview.

##### `handleDeleteClick(filename: string, e: React.MouseEvent)`
- Prevents event bubbling with `e.stopPropagation()`
- Sets `deleteConfirm` state to trigger modal display

##### `confirmDelete()`
- Calls `delete_entry` Tauri command with pending filename
- Clears selection if deleted entry was active
- Reloads entry list
- Closes modal by clearing `deleteConfirm` state

##### `cancelDelete()`
- Closes modal by clearing `deleteConfirm` state
- Entry remains unchanged

### UI Components

### Voice Hooks (`src/hooks/`)

#### `useWhisperModel.ts`

```typescript
function useWhisperModel() {
  return {
    isModelReady: boolean,      // Model downloaded and valid
    isDownloading: boolean,     // Download in progress
    downloadProgress: number,   // 0-100
    downloadModel: () => void,  // Trigger download
    error: string | null,
  };
}
```

- Checks model status on mount
- Listens for `whisper-download-progress` events
- Triggers download via `download_whisper_model` command

#### `useVoiceRecording.ts`

```typescript
function useVoiceRecording({ onTranscription, isModelReady }) {
  return {
    state: 'idle' | 'recording' | 'transcribing',
    isRecording: boolean,
    isTranscribing: boolean,
    recordingDuration: number,   // Seconds
    formattedDuration: string,   // "0:00" format
    startRecording: () => void,
    stopRecording: () => void,
    error: string | null,
  };
}
```

- Manages recording state machine
- Tracks recording duration with timer
- Registers global shortcut `Cmd+Shift+R` (hold to record)
- Calls `onTranscription` callback with result

#### `useAudioImport.ts`

```typescript
function useAudioImport({ onTranscription, isModelReady }) {
  return {
    importAudioFile: () => void,  // Opens file dialog
    isImporting: boolean,
    error: string | null,
  };
}
```

- Opens file dialog with audio filters (mp3, m4a, wav, ogg, flac, aac)
- Calls `transcribe_audio_file` command
- Calls `onTranscription` callback with result

#### Layout Structure

```
app (flexbox container)
├── sidebar
│   ├── sidebar-header
│   │   ├── h1 (Journal)
│   │   ├── new-entry-btn
│   │   └── import-audio-btn          # NEW
│   └── entry-list
│       └── entry-item (multiple)
│           ├── entry-item-content
│           │   ├── entry-item-title
│           │   └── entry-item-date
│           └── delete-btn (trash icon SVG)
├── editor-container
│   └── blog-view
│       ├── blog-header
│       │   ├── blog-title (input)
│       │   ├── blog-date (input)
│       │   ├── saving-indicator
│       │   ├── voice-controls         # NEW
│       │   │   ├── download-model-btn (if model not ready)
│       │   │   ├── record-btn (if model ready)
│       │   │   └── shortcut-hint
│       │   └── voice-error (conditional)
│       └── blog-content
│           └── blog-editor (textarea)
└── modal-overlay (conditional)
    └── modal
        ├── modal-icon (trash SVG)
        ├── modal-title
        ├── modal-message
        └── modal-buttons
            ├── modal-btn-cancel
            └── modal-btn-delete
```

#### Sidebar Entry Component

```typescript
<div
  key={entry.filename}
  className={`entry-item ${selectedEntry === entry.filename ? "active" : ""}`}
  onClick={() => handleEntryClick(entry.filename)}
>
  <div className="entry-item-content">
    <div className="entry-item-title">
      {entry.title || entry.date || entry.filename.replace(/\.md$/, "")}
    </div>
    {entry.date && (
      <div className="entry-item-date">{entry.date}</div>
    )}
  </div>
  <button
    className="delete-btn"
    onClick={(e) => handleDeleteClick(entry.filename, e)}
    title="Delete entry"
  >
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polyline points="3 6 5 6 21 6"></polyline>
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
      <line x1="10" y1="11" x2="10" y2="17"></line>
      <line x1="14" y1="11" x2="14" y2="17"></line>
    </svg>
  </button>
</div>
```

**Display Logic:**
- Primary: Show title if exists
- Fallback 1: Show date if title is empty
- Fallback 2: Show filename without `.md` extension

#### Editor Component

```typescript
<div className="blog-view">
  <div className="blog-header">
    <input
      type="text"
      className="blog-title"
      value={title}
      onChange={handleTitleChange}
      placeholder="Untitled"
    />
    <input
      type="text"
      className="blog-date"
      value={date}
      onChange={handleDateChange}
      placeholder="Date"
    />
    {isSaving && <div className="saving-indicator">Saving...</div>}
  </div>
  <div className="blog-content">
    <textarea
      className="blog-editor"
      value={content}
      onChange={handleContentChange}
      placeholder="Start writing your thoughts..."
    />
  </div>
</div>
```

#### Delete Confirmation Modal

A custom React modal replaces the native browser `confirm()` dialog (which doesn't work in Tauri's webview).

```typescript
{deleteConfirm && (
  <div className="modal-overlay" onClick={cancelDelete}>
    <div className="modal" onClick={(e) => e.stopPropagation()}>
      <div className="modal-icon">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#ff3b30" strokeWidth="2">
          <polyline points="3 6 5 6 21 6"></polyline>
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
        </svg>
      </div>
      <h2 className="modal-title">Delete Entry?</h2>
      <p className="modal-message">
        Are you sure you want to delete this entry? This action cannot be undone.
      </p>
      <div className="modal-buttons">
        <button className="modal-btn modal-btn-cancel" onClick={cancelDelete}>
          Cancel
        </button>
        <button className="modal-btn modal-btn-delete" onClick={confirmDelete}>
          Delete
        </button>
      </div>
    </div>
  </div>
)}
```

**Modal Features:**
- Semi-transparent overlay backdrop
- Click outside modal to cancel
- Trash icon in danger color (red)
- Clear warning message
- Cancel and Delete buttons
- Delete button styled in danger color

## Styling (CSS)

### File Location: `src/App.css`

### Design System

**Colors:**
- Primary: `#007aff` (macOS blue)
- Primary Hover: `#0051d5`
- Danger: `#ff3b30` (macOS red)
- Background: `#fafafa`
- Sidebar: `#f5f5f5`
- Text: `#333`, `#666`, `#999`
- Borders: `#ddd`, `#e5e5e5`

**Typography:**
- Font Family: `-apple-system, BlinkMacSystemFont, 'Segoe UI', ...`
- Title: 42px, weight 700
- Date: 16px, weight 400
- Body: 18px, line-height 1.8
- Sidebar Title: 14px
- Sidebar Date: 12px

### Key Layout Rules

#### App Container
```css
.app {
  display: flex;
  height: 100vh;
  overflow: hidden;
}
```

#### Sidebar
```css
.sidebar {
  width: 250px;
  background-color: #f5f5f5;
  border-right: 1px solid #ddd;
  display: flex;
  flex-direction: column;
}

.entry-list {
  flex: 1;
  overflow-y: auto;  /* Automatic scrolling */
}
```

#### Entry Item States

**Default:**
```css
.entry-item {
  padding: 12px 16px;
  cursor: pointer;
  border-bottom: 1px solid #e5e5e5;
  transition: all 0.2s;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
}
```

**Hover:**
```css
.entry-item:hover {
  background-color: #e8e8e8;
}
```

**Active (Selected):**
```css
.entry-item.active {
  background-color: #007aff;
  border-left: 3px solid #0051d5;
  padding-left: 13px;
}

.entry-item.active .entry-item-title {
  color: white;
  font-weight: 600;
}

.entry-item.active .entry-item-date {
  color: rgba(255, 255, 255, 0.85);
}
```

#### Delete Button

Uses an SVG trash icon instead of text character for better visual consistency.

**Hidden by Default:**
```css
.delete-btn {
  background: transparent;
  border: none;
  color: #999;
  cursor: pointer;
  padding: 4px;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  opacity: 0;  /* Hidden until hover */
  transition: all 0.2s;
  position: relative;
  z-index: 10;
  pointer-events: auto;
}
```

**Show on Hover:**
```css
.entry-item:hover .delete-btn {
  opacity: 1;
}

.delete-btn:hover {
  background-color: #ff3b30;
  color: white;
}
```

**Active Entry Delete Button:**
```css
.entry-item.active .delete-btn {
  color: white;
}

.entry-item.active .delete-btn:hover {
  background-color: rgba(255, 255, 255, 0.2);
}
```

#### Blog View (Editor)

**Container:**
```css
.blog-view {
  max-width: 800px;
  width: 100%;
  margin: 0 auto;  /* Centered */
  padding: 60px 40px;
  background-color: white;
  min-height: 100%;
}
```

**Title Input:**
```css
.blog-title {
  width: 100%;
  border: none;
  outline: none;
  font-size: 42px;
  font-weight: 700;
  line-height: 1.2;
  margin-bottom: 16px;
  color: #1a1a1a;
  background: transparent;
}
```

**Date Input:**
```css
.blog-date {
  width: 100%;
  border: none;
  outline: none;
  font-size: 16px;
  font-weight: 400;
  color: #666;
  background: transparent;
  margin-bottom: 12px;
}
```

**Content Editor:**
```css
.blog-editor {
  width: 100%;
  min-height: 500px;
  border: none;
  outline: none;
  font-size: 18px;
  line-height: 1.8;
  color: #333;
  resize: none;
  background: transparent;
}
```

### Text Truncation

```css
.entry-item-title,
.entry-item-date {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
```

### Delete Confirmation Modal

```css
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: white;
  border-radius: 12px;
  padding: 32px;
  max-width: 400px;
  width: 90%;
  text-align: center;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.modal-icon {
  margin-bottom: 16px;
}

.modal-title {
  font-size: 20px;
  font-weight: 600;
  color: #1a1a1a;
  margin-bottom: 8px;
}

.modal-message {
  font-size: 14px;
  color: #666;
  margin-bottom: 24px;
  line-height: 1.5;
}

.modal-buttons {
  display: flex;
  gap: 12px;
  justify-content: center;
}

.modal-btn {
  padding: 10px 24px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
}

.modal-btn-cancel {
  background-color: #e5e5e5;
  color: #333;
}

.modal-btn-cancel:hover {
  background-color: #d5d5d5;
}

.modal-btn-delete {
  background-color: #ff3b30;
  color: white;
}

.modal-btn-delete:hover {
  background-color: #e0352b;
}
```

### Voice Controls Styling

```css
.voice-controls {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 16px;
}

.record-btn {
  padding: 12px 24px;
  background-color: #007aff;
  color: white;
  border: none;
  border-radius: 24px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.2s;
}

.record-btn.recording {
  background-color: #ff3b30;
  animation: pulse 1.5s ease-in-out infinite;
}

.record-btn.recording::before {
  content: '';
  width: 10px;
  height: 10px;
  background-color: white;
  border-radius: 50%;
  animation: blink 1s ease-in-out infinite;
}

.import-audio-btn {
  width: 100%;
  padding: 8px 16px;
  background-color: #5856d6;
  color: white;
  border-radius: 6px;
  margin-top: 8px;
}

.download-model-btn {
  padding: 12px 24px;
  background-color: #34c759;
  color: white;
  border-radius: 24px;
}

.shortcut-hint {
  font-size: 12px;
  color: #999;
}

.voice-error {
  margin-top: 12px;
  padding: 8px 12px;
  background-color: #ffebee;
  color: #c62828;
  border-radius: 6px;
}
```

## Build Instructions

### Prerequisites

1. **Node.js**: Version 20.19+ or 22.12+ (Vite requirement)
2. **Rust**: Install via `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. **Tauri CLI**: Installed via npm (included in dependencies)

### Initial Setup

```bash
# Clone or create project directory
mkdir journal-app
cd journal-app

# Initialize Tauri project
npm create tauri-app@latest

# Project setup answers:
# - Project name: journal-app
# - Package manager: npm
# - UI template: React + TypeScript
# - Add Vite plugin: Yes
```

### Install Dependencies

```bash
# Frontend dependencies
npm install

# Tauri plugins (add to src-tauri/Cargo.toml)
# Already specified in Cargo.toml dependencies section

# Additional Rust dependencies
cd src-tauri
cargo add chrono dirs regex
cd ..
```

### Project Configuration

#### Update `package.json`

```json
{
  "name": "journal-app",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "react": "^19.1.0",
    "react-dom": "^19.1.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@types/react": "^19.1.0",
    "@types/react-dom": "^19.1.0",
    "@vitejs/plugin-react": "^4.6.0",
    "typescript": "~5.8.3",
    "vite": "^7.0.4"
  }
}
```

#### Update `src-tauri/Cargo.toml`

```toml
[package]
name = "journal-app"
version = "0.1.0"
description = "A Tauri App"
authors = ["you"]
edition = "2021"

[lib]
name = "journal_app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-fs = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
dirs = "5.0"
regex = "1"
```

### Development Workflow

```bash
# Start development server
npm run tauri dev

# Build for production
npm run tauri build

# Clean Rust build artifacts (if needed)
cd src-tauri && cargo clean && cd ..
```

### Important Notes

1. **Hot Reload**: Frontend changes auto-reload, but Rust changes require app restart
2. **New Tauri Commands**: Always restart the app after adding new `#[tauri::command]` functions
3. **Data Directory**: Created automatically at `~/Documents/Project Data Files/Journal/`
4. **Permissions**: Tauri 2 uses filesystem plugin for file access (already configured)

## User Workflows

### Creating a New Entry

1. Click "+ New Entry" button
2. File created with current date in frontmatter
3. Entry opens in editor
4. Start typing in title, date, or content fields
5. Auto-saves after 1 second of inactivity
6. Filename updates based on title/date

### Editing an Entry

1. Click entry in sidebar
2. Entry loads with title, date, content
3. Edit any field
4. Changes auto-save after 1 second
5. "Saving..." indicator appears during save
6. Filename updates if title changes

### Deleting an Entry

1. Hover over entry in sidebar
2. Trash icon button appears on the right
3. Click trash icon
4. Confirmation modal appears with:
   - Trash icon in red
   - "Delete Entry?" title
   - Warning: "This action cannot be undone"
   - Cancel and Delete buttons
5. Click Delete to confirm (or Cancel to abort)
6. Entry removed from sidebar and filesystem
7. If deleted entry was open, editor clears to empty state
8. Click outside modal also cancels deletion

### Organizing Entries

- Entries automatically sorted by date (newest first)
- Entries without valid dates sorted by filename
- Scroll sidebar when entries exceed viewport

### Voice Features

#### First-Time Setup (Download Whisper Model)

1. Select or create a journal entry
2. In the editor header, click green "Download Whisper Model" button
3. Wait for download to complete (~142MB, progress shown)
4. Button changes to "Hold to Record" when ready

#### Push-to-Talk Recording (Button)

1. Select an entry to record into
2. Click and **hold** the "Hold to Record" button
3. Speak while holding the button
4. Recording timer shows duration (e.g., "0:15")
5. **Release** the button to stop recording
6. Button shows "Transcribing..." while processing
7. Transcribed text appends to entry content

#### Push-to-Talk Recording (Keyboard)

1. Select an entry to record into
2. **Hold** `Cmd+Shift+R` (works globally, even when app not focused)
3. Speak while holding the keys
4. **Release** to stop and transcribe
5. Text appends to entry content

#### Import Voice Memo

1. Select an entry to import into
2. Click "Import Audio" button in sidebar
3. Select audio file (MP3, M4A, WAV, OGG, FLAC, AAC)
4. Button shows "Transcribing..." while processing
5. Transcribed text appends to entry content

## Error Handling

### Backend Errors

All Tauri commands return `Result<T, String>`:

```rust
// Success
Ok(value)

// Error with message
Err("Error description".to_string())
```

### Frontend Error Handling

```typescript
try {
  await invoke("command_name", { params });
} catch (error) {
  console.error("Operation failed:", error);
  // Some operations also show alert() to user
}
```

### Common Issues and Solutions

**Issue:** Rust command not found
**Solution:** Restart app after adding new commands

**Issue:** Build artifacts from old location
**Solution:** Run `cargo clean` in src-tauri directory

**Issue:** Vite Node.js version warning
**Solution:** Upgrade to Node.js 20.19+ or 22.12+

**Issue:** File already exists error
**Solution:** Change title to unique value

**Issue:** Date sorting not working
**Solution:** Use format "Month Day, Year" (e.g., "January 1, 2026")

**Issue:** Native browser dialogs (confirm, alert, prompt) don't work
**Solution:** Use custom React modals instead (Tauri webview doesn't support native dialogs)

## Testing Checklist

### Basic Functionality
- [ ] Create new entry
- [ ] Entry appears in sidebar with date
- [ ] Edit title and verify filename changes
- [ ] Edit date field
- [ ] Edit content
- [ ] Verify auto-save after 1 second
- [ ] Close and reopen app - changes persisted
- [ ] Create multiple entries
- [ ] Verify newest entries appear first

### Delete Functionality
- [ ] Hover over entry shows trash icon button
- [ ] Click trash icon shows confirmation modal
- [ ] Modal displays trash icon, title, warning message
- [ ] Cancel button closes modal, keeps entry
- [ ] Click outside modal closes it, keeps entry
- [ ] Delete button removes entry
- [ ] Deleting active entry clears editor
- [ ] Deleting inactive entry keeps editor open
- [ ] Deleted file removed from filesystem

### Edge Cases
- [ ] Create entry with no title (uses date as filename)
- [ ] Create entry with title (uses title as filename)
- [ ] Change from titled to untitled
- [ ] Change from untitled to titled
- [ ] Enter duplicate title (should error)
- [ ] Special characters in title
- [ ] Very long title
- [ ] Empty content saves correctly
- [ ] Rapid typing triggers single save (debounced)

### UI/UX
- [ ] Sidebar scrolls with many entries
- [ ] Active entry highlighted in blue
- [ ] Trash icon hidden until hover
- [ ] Trash icon visible on active entry (white)
- [ ] Delete modal centered with overlay
- [ ] Delete modal has proper styling
- [ ] Title input borderless and seamless
- [ ] Content area centered at 800px
- [ ] Placeholder text visible when empty
- [ ] Saving indicator appears during save

### Voice Features
- [ ] "Download Whisper Model" button appears on first run
- [ ] Download progress shows percentage
- [ ] After download, "Hold to Record" button appears
- [ ] Hold button starts recording (red with pulse animation)
- [ ] Recording timer displays correctly (0:00 format)
- [ ] Release button stops recording
- [ ] "Transcribing..." state shows during processing
- [ ] Transcribed text appends to entry content
- [ ] `Cmd+Shift+R` shortcut starts recording when held
- [ ] `Cmd+Shift+R` works even when app not focused
- [ ] "Import Audio" button opens file dialog
- [ ] MP3 files import and transcribe correctly
- [ ] M4A files import and transcribe correctly
- [ ] WAV files import and transcribe correctly
- [ ] Buttons disabled when no entry selected
- [ ] Error messages display in red banner
- [ ] First transcription loads model (brief delay)
- [ ] Subsequent transcriptions are faster (model cached)
- [ ] macOS microphone permission prompt appears

## Future Enhancements

1. **Security**: Touch ID, encryption, password protection
2. **Search**: Full-text search across entries
3. **Tags**: Categorization and filtering
4. **Export**: Bulk export to PDF, HTML
5. **Themes**: Dark mode support
6. **Markdown**: Live preview with formatting
7. **Attachments**: Embed images and files
8. **Cloud Sync**: Optional iCloud/Dropbox sync
9. **Templates**: Pre-defined entry templates
10. **Calendar**: Date-based navigation
11. **Voice Enhancements**: Configurable keyboard shortcut, different Whisper model sizes, real-time transcription preview

## File System Details

### Directory Structure

```
~/Documents/Project Data Files/Journal/
├── models/
│   └── ggml-base.en.bin        # Whisper model (~142MB)
├── January 1, 2026.md
├── My First Entry.md
├── Weekend Thoughts.md
└── December 31, 2025.md
```

### File Permissions

Files created with default user permissions (readable/writable by user only).

### Backup Recommendation

Since files are plain markdown, users can backup by:
1. Time Machine (automatic macOS backup)
2. Copy folder to external drive
3. Cloud sync via Dropbox/iCloud (manual)
4. Git repository (for version control)

## API Reference

### Tauri Commands (Rust → TypeScript)

```typescript
// List all entries with metadata
invoke<EntryInfo[]>("list_entries")

// Read specific entry content
invoke<string>("read_entry", { filename: "Entry.md" })

// Save entry (legacy - prefer update_entry_metadata)
invoke<void>("save_entry", {
  filename: "Entry.md",
  content: "---\ntitle: ...\n---\n..."
})

// Create new entry
invoke<string>("create_entry")

// Update entry with metadata sync
invoke<string>("update_entry_metadata", {
  filename: "Old.md",
  title: "New Title",
  date: "January 1, 2026",
  content: "Content here"
})

// Rename entry
invoke<void>("rename_entry", {
  old_filename: "Old.md",
  new_filename: "New.md"
})

// Delete entry
invoke<void>("delete_entry", { filename: "Entry.md" })
```

## Deployment

### macOS App Bundle

```bash
# Build production app
npm run tauri build

# Output location
src-tauri/target/release/bundle/macos/journal-app.app
```

### Distribution Options

1. **Direct Distribution**: Share .app bundle
2. **DMG**: Create disk image for installation
3. **Mac App Store**: Requires Apple Developer account and code signing
4. **Homebrew**: Create formula for package manager installation

### Code Signing (Future)

For distribution outside Mac App Store:
1. Obtain Apple Developer certificate
2. Configure in tauri.conf.json
3. Notarize app with Apple

## Development Tips

### Debugging

**Frontend:**
- Open DevTools in app window
- Use `console.log()` for debugging
- React DevTools available

**Backend:**
- Use `println!()` or `eprintln!()` in Rust
- Output appears in terminal running `npm run tauri dev`
- Use `dbg!()` macro for variable inspection

### Performance

- Auto-save debounce prevents excessive writes
- Frontmatter parsing cached in `list_entries`
- Minimal re-renders with proper React hooks

### Best Practices

1. Always validate file existence before operations
2. Use `Result<T, String>` for error handling
3. Sanitize filenames (no special chars except in content)
4. Test with large numbers of entries (100+)
5. Test with very long content (10,000+ words)

## Version History

### v1.1 (Current)
- Voice memo import (MP3, M4A, WAV, OGG, FLAC, AAC)
- Push-to-talk dictation with local Whisper transcription
- Global keyboard shortcut (Cmd+Shift+R)
- On-demand Whisper model download

### v1.0
- Initial release
- Core journaling functionality
- Blog-style editor
- Delete functionality with confirmation
- Auto-save
- Date-based sorting

## Credits

Built with:
- Tauri 2: Desktop app framework
- React 19: UI library
- TypeScript: Type safety
- Rust: Backend logic
- Vite 7: Build tool
- chrono: Date parsing
- regex: Frontmatter parsing

## License

Private project - all rights reserved.

---

**End of Documentation**

This documentation is designed to be comprehensive enough for any AI coding agent (Claude Code, Cursor, etc.) to rebuild the project from scratch or make modifications while maintaining consistency with the original design.
