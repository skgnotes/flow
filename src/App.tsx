import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

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

function parseFrontmatter(text: string): EntryMetadata {
  const frontmatterRegex = /^---\n([\s\S]*?)\n---\n([\s\S]*)$/;
  const match = text.match(frontmatterRegex);

  if (!match) {
    // No frontmatter, treat entire text as content
    return { title: "", date: "", content: text };
  }

  const frontmatter = match[1];
  const content = match[2].trim();

  const titleMatch = frontmatter.match(/^title:\s*(.*)$/m);
  const dateMatch = frontmatter.match(/^date:\s*(.*)$/m);

  return {
    title: titleMatch ? titleMatch[1].trim() : "",
    date: dateMatch ? dateMatch[1].trim() : "",
    content: content,
  };
}

function App() {
  const [entries, setEntries] = useState<EntryInfo[]>([]);
  const [selectedEntry, setSelectedEntry] = useState<string | null>(null);
  const [title, setTitle] = useState<string>("");
  const [date, setDate] = useState<string>("");
  const [content, setContent] = useState<string>("");
  const [isSaving, setIsSaving] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

  // Load all entries on mount
  useEffect(() => {
    loadEntries();
  }, []);

  const loadEntries = async () => {
    try {
      const entryList = await invoke<EntryInfo[]>("list_entries");
      setEntries(entryList);
    } catch (error) {
      console.error("Failed to load entries:", error);
    }
  };

  const loadEntry = async (filename: string) => {
    try {
      const entryContent = await invoke<string>("read_entry", { filename });
      setSelectedEntry(filename);

      const parsed = parseFrontmatter(entryContent);
      setTitle(parsed.title);
      setDate(parsed.date);
      setContent(parsed.content);
    } catch (error) {
      console.error("Failed to load entry:", error);
    }
  };

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

      // If filename changed, update state
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

  // Auto-save on any field change (debounced)
  useEffect(() => {
    if (!selectedEntry) return;

    const timeout = setTimeout(() => {
      saveEntry();
    }, 1000);

    return () => clearTimeout(timeout);
  }, [title, date, content, selectedEntry, saveEntry]);

  const createNewEntry = async () => {
    try {
      const newFilename = await invoke<string>("create_entry");
      await loadEntries();
      await loadEntry(newFilename);
    } catch (error) {
      console.error("Failed to create entry:", error);
    }
  };

  const handleEntryClick = (filename: string) => {
    loadEntry(filename);
  };

  const handleTitleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setTitle(e.target.value);
  };

  const handleDateChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setDate(e.target.value);
  };

  const handleContentChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setContent(e.target.value);
  };

  const handleDeleteClick = (filename: string, e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDeleteConfirm(filename);
  };

  const confirmDelete = async () => {
    if (!deleteConfirm) return;

    try {
      await invoke("delete_entry", { filename: deleteConfirm });

      // If we deleted the currently selected entry, clear selection
      if (selectedEntry === deleteConfirm) {
        setSelectedEntry(null);
        setTitle("");
        setDate("");
        setContent("");
      }

      // Reload the entry list
      await loadEntries();
    } catch (error) {
      console.error("Failed to delete entry:", error);
    } finally {
      setDeleteConfirm(null);
    }
  };

  const cancelDelete = () => {
    setDeleteConfirm(null);
  };

  return (
    <div className="app">
      <div className="sidebar">
        <div className="sidebar-header">
          <h1>✨ Flow</h1>
          <button className="new-entry-btn" onClick={createNewEntry}>
            + New Entry
          </button>
        </div>
        <div className="entry-list">
          {entries.map((entry) => (
            <div
              key={entry.filename}
              className={`entry-item ${selectedEntry === entry.filename ? "active" : ""
                }`}
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
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="3 6 5 6 21 6"></polyline>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                  <line x1="10" y1="11" x2="10" y2="17"></line>
                  <line x1="14" y1="11" x2="14" y2="17"></line>
                </svg>
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="editor-container">
        {selectedEntry ? (
          <>
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
          </>
        ) : (
          <div className="empty-state">
            Select an entry or create a new one to start journaling
          </div>
        )}
      </div>

      {deleteConfirm && (
        <div className="modal-overlay" onClick={cancelDelete}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-icon">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#ff3b30" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
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
    </div>
  );
}

export default App;
