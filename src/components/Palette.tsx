import { useCallback, useEffect, useRef, useState } from "react";
import { api, copyToClipboard, errorMessage, events } from "../lib/ipc";
import { applyTheme } from "../lib/theme";
import type { ChatMessage, CommandManifest, HistoryEntry, OpenPayload, PlatformInfo, Progress } from "../lib/types";
import { ChatView, type UiMessage } from "./ChatView";
import { CommandList } from "./CommandList";
import { Footer } from "./Footer";
import { HistoryView } from "./HistoryView";
import { InputView } from "./InputView";
import { INITIAL_PROGRESS, ResultView, type RunProgress, type RunState } from "./ResultView";

function applyProgress(p: RunProgress, e: Progress): RunProgress {
  switch (e.type) {
    case "connecting":
      return { ...p, stage: "connecting", provider: e.provider, model: e.model };
    case "thinking":
      return { ...p, stage: "thinking" };
    case "writing":
      return { ...p, stage: "writing" };
    case "delta":
      return { ...p, text: p.text + e.text };
  }
}

type View =
  | { name: "search" }
  | { name: "history" }
  | { name: "chat" }
  | { name: "input"; cmd: CommandManifest; initial: string }
  | { name: "run"; cmd: CommandManifest; input: string | null; fromSelection: boolean; state: RunState };

export function Palette() {
  const [view, setView] = useState<View>({ name: "search" });
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<CommandManifest[]>([]);
  const [selected, setSelected] = useState(0);
  const [selection, setSelection] = useState<string | null>(null);
  const [platform, setPlatform] = useState<PlatformInfo | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [progress, setProgress] = useState<RunProgress>(INITIAL_PROGRESS);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [slashArgs, setSlashArgs] = useState("");
  const [chatMessages, setChatMessages] = useState<UiMessage[]>([]);
  const [chatBusy, setChatBusy] = useState(false);
  const [chatError, setChatError] = useState<string | null>(null);
  const [chatContext, setChatContext] = useState<string | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);
  const searchSeq = useRef(0);
  const runSeq = useRef(0);
  const autoReplaceRef = useRef(false);
  const platformRef = useRef<PlatformInfo | null>(null);
  platformRef.current = platform;

  // ---- search -------------------------------------------------------------
  const refresh = useCallback(async (q: string) => {
    const seq = ++searchSeq.current;
    try {
      const slash = q.trimStart().startsWith("/");
      const { items: found, args } = slash ? await api.searchSlash(q) : { items: await api.search(q), args: "" };
      if (seq === searchSeq.current) {
        setItems(found);
        setSlashArgs(args);
        setSelected(0);
      }
    } catch (e) {
      console.error("search failed", e);
    }
  }, []);

  useEffect(() => {
    void refresh(query);
  }, [query, refresh]);

  // ---- running commands ---------------------------------------------------
  const close = useCallback(async () => {
    runSeq.current++;
    await api.hidePalette();
  }, []);

  // Put `text` back where the selection came from. On Wayland the clipboard is only
  // writable from the focused window, so fill it before the palette hides.
  const pasteBack = useCallback(async (text: string) => {
    if (platformRef.current?.session === "wayland") await copyToClipboard(text);
    await api.replaceSelection(text);
  }, []);

  const openHistory = useCallback(async () => {
    try {
      setHistory(await api.listHistory());
      setNotice(null);
      setView({ name: "history" });
    } catch (e) {
      setNotice(errorMessage(e));
    }
  }, []);

  // ---- chat ---------------------------------------------------------------
  const runChat = useCallback(async (messages: UiMessage[]) => {
    const seq = ++runSeq.current;
    setProgress(INITIAL_PROGRESS);
    setChatError(null);
    setChatBusy(true);
    try {
      const payload: ChatMessage[] = messages.map(({ role, content }) => ({ role, content }));
      const answer = await api.chat(payload, seq);
      if (seq !== runSeq.current) return; // cancelled, or the palette was reopened
      setChatMessages([...messages, { role: "assistant", content: answer }]);
      setChatBusy(false);
    } catch (e) {
      if (seq !== runSeq.current) return;
      setChatError(errorMessage(e));
      setChatBusy(false);
    }
  }, []);

  const chatRef = useRef<{ messages: UiMessage[]; context: string | null }>({ messages: [], context: null });
  chatRef.current = { messages: chatMessages, context: chatContext };

  const sendChat = useCallback(
    (text: string) => {
      const { messages, context } = chatRef.current;
      // The selected text rides along with the first message only.
      const first = messages.length === 0 && context;
      const content = first ? `Selected text:\n"""\n${context}\n"""\n\n${text}` : text;
      const next: UiMessage[] = [...messages, { role: "user", content, shown: text }];
      if (first) setChatContext(null);
      setChatMessages(next);
      void runChat(next);
    },
    [runChat],
  );

  const openChat = useCallback(
    (question: string | null) => {
      setNotice(null);
      setView({ name: "chat" });
      if (question) sendChat(question);
    },
    [sendChat],
  );

  const run = useCallback(async (cmd: CommandManifest, input: string | null, fromSelection: boolean) => {
    const seq = ++runSeq.current;
    setNotice(null);
    setProgress(INITIAL_PROGRESS);
    setView({ name: "run", cmd, input, fromSelection, state: { kind: "running" } });
    try {
      const out = await api.execute(cmd.id, input, seq);
      if (seq !== runSeq.current) return; // cancelled or superseded
      if (out.type === "text" && autoReplaceRef.current && cmd.replaces_selection && fromSelection) {
        await pasteBack(out.text);
      } else if (out.type === "text") {
        setView({ name: "run", cmd, input, fromSelection, state: { kind: "text", text: out.text } });
      } else if (out.type === "action" && out.action.type === "show-history") {
        await openHistory();
      } else if (out.type === "action" && out.action.type === "open-chat") {
        openChat(out.action.question);
      } else {
        await api.hidePalette();
      }
    } catch (e) {
      if (seq !== runSeq.current) return;
      setView({ name: "run", cmd, input, fromSelection, state: { kind: "error", message: errorMessage(e) } });
    }
  }, [pasteBack, openHistory, openChat]);

  const start = useCallback(
    (cmd: CommandManifest, sel: string | null, args = "") => {
      if (cmd.input === "none") return void run(cmd, null, false);
      if (cmd.input === "optional-text") return void run(cmd, args || null, false);
      // Text typed after a slash command wins over the captured selection.
      if (args) return void run(cmd, args, false);
      if (cmd.input === "selection" && sel) return void run(cmd, sel, true);
      setView({ name: "input", cmd, initial: sel ?? "" });
    },
    [run],
  );

  const backToSearch = useCallback(() => {
    runSeq.current++; // abandon any in-flight request
    setNotice(null);
    setView({ name: "search" });
  }, []);

  // ---- lifecycle ----------------------------------------------------------
  useEffect(() => {
    api
      .getConfig()
      .then((c) => {
        applyTheme(c.theme);
        autoReplaceRef.current = c.auto_replace;
      })
      .catch(console.error);
    const unlisten = Promise.all([
      events.onConfigChanged((c) => {
        applyTheme(c.theme);
        autoReplaceRef.current = c.auto_replace;
        void refresh("");
      }),
      events.onProgress(({ run_id, event }) => {
        if (run_id === runSeq.current) setProgress((prev) => applyProgress(prev, event));
      }),
      events.onPaletteOpen((p: OpenPayload) => {
        runSeq.current++;
        setNotice(null);
        setQuery("");
        setSelection(p.selection);
        setChatMessages([]);
        setChatBusy(false);
        setChatError(null);
        setChatContext(p.selection);
        setView({ name: "search" });
        api.platformInfo().then(setPlatform).catch(console.error);
        void refresh("");
        setTimeout(() => inputRef.current?.focus(), 0);
        if (p.run) {
          api
            .listCommands()
            .then((all) => {
              const cmd = all.find((c) => c.id === p.run);
              if (cmd) start(cmd, p.selection);
              else setNotice(`Unknown command: ${p.run}`);
            })
            .catch(console.error);
        }
      }),
    ]);
    api.platformInfo().then(setPlatform).catch(console.error);
    return () => void unlisten.then((fns) => fns.forEach((f) => f()));
  }, [refresh, start]);

  useEffect(() => {
    if (view.name === "search") inputRef.current?.focus();
  }, [view.name]);

  // ---- result actions -----------------------------------------------------
  const resultText = view.name === "run" && view.state.kind === "text" ? view.state.text : null;
  const canReplace = view.name === "run" && view.cmd.replaces_selection && view.fromSelection;

  const copy = useCallback(async () => {
    if (resultText === null) return;
    try {
      await copyToClipboard(resultText);
      setNotice("Copied to clipboard");
      setTimeout(() => void close(), 350);
    } catch (e) {
      setNotice(errorMessage(e));
    }
  }, [resultText, close]);

  const replace = useCallback(async () => {
    if (resultText === null || !canReplace) return;
    try {
      if (platform?.auto_replace) {
        await pasteBack(resultText);
      } else {
        await copyToClipboard(resultText);
        setNotice("Copied — close Keymint (Esc) and paste with Ctrl+V");
      }
    } catch (e) {
      setNotice(errorMessage(e));
    }
  }, [resultText, canReplace, platform, pasteBack]);

  const retry = useCallback(() => {
    if (view.name === "run" && view.state.kind !== "running") void run(view.cmd, view.input, view.fromSelection);
  }, [view, run]);

  const editInput = useCallback(() => {
    if (view.name === "run") setView({ name: "input", cmd: view.cmd, initial: view.input ?? "" });
  }, [view]);

  // Keyboard shortcuts for the result view (the search and input views handle their own keys).
  useEffect(() => {
    if (view.name !== "run") return;
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (e.key === "Escape") backToSearch();
      else if (e.key === "Enter" && mod) void replace();
      else if (e.key === "Enter") void copy();
      else if (mod && e.key.toLowerCase() === "r") retry();
      else if (mod && e.key.toLowerCase() === "e") editInput();
      else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [view.name, backToSearch, replace, copy, retry, editInput]);

  const newChat = useCallback(() => {
    runSeq.current++;
    setChatMessages([]);
    setChatBusy(false);
    setChatError(null);
    setChatContext(selection);
    setNotice(null);
  }, [selection]);

  const retryChat = useCallback(() => void runChat(chatRef.current.messages), [runChat]);

  const chatCopy = useCallback(async (text: string) => {
    try {
      await copyToClipboard(text);
      setNotice("Copied to clipboard");
    } catch (e) {
      setNotice(errorMessage(e));
    }
  }, []);

  const leaveChat = useCallback(() => {
    // Cancelling a pending answer keeps the question so it can be retried.
    if (chatBusy) {
      runSeq.current++;
      setChatBusy(false);
      setChatError("Cancelled");
    } else {
      backToSearch();
    }
  }, [chatBusy, backToSearch]);

  // ---- history ------------------------------------------------------------
  const historyCopy = useCallback(
    async (text: string) => {
      try {
        await copyToClipboard(text);
        setNotice("Copied to clipboard");
        setTimeout(() => void close(), 350);
      } catch (e) {
        setNotice(errorMessage(e));
      }
    },
    [close],
  );

  const historyPaste = useCallback(
    async (text: string) => {
      try {
        await pasteBack(text);
      } catch (e) {
        setNotice(errorMessage(e));
      }
    },
    [pasteBack],
  );

  const historyDelete = useCallback(async (id: number) => {
    try {
      await api.deleteHistoryEntry(id);
      setHistory((h) => h.filter((entry) => entry.id !== id));
    } catch (e) {
      setNotice(errorMessage(e));
    }
  }, []);

  // ---- search view keys ---------------------------------------------------
  const onSearchKey = (e: React.KeyboardEvent) => {
    const last = items.length - 1;
    if (e.key === "ArrowDown" || (e.ctrlKey && e.key === "n")) setSelected((s) => Math.min(s + 1, last));
    else if (e.key === "ArrowUp" || (e.ctrlKey && e.key === "p")) setSelected((s) => Math.max(s - 1, 0));
    else if (e.key === "Enter") {
      const cmd = items[selected];
      if (cmd) start(cmd, selection, slashArgs);
    } else if (e.key === "Tab" && query.trimStart().startsWith("/") && items[selected]?.slash) {
      setQuery(`/${items[selected].slash} `);
    } else if (e.ctrlKey && e.key === "h") {
      void openHistory();
    } else if (e.key === "Escape") {
      if (query) setQuery("");
      else void close();
    } else return;
    e.preventDefault();
  };

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-2xl border border-line bg-surface">
      {view.name === "search" && (
        <>
          <div className="flex items-center gap-2 border-b border-line px-4 py-3">
            <input
              ref={inputRef}
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={onSearchKey}
              placeholder="Search commands…"
              spellCheck={false}
              autoComplete="off"
              className="min-w-0 flex-1 bg-transparent text-base outline-none placeholder:text-muted"
            />
            {selection && (
              <button
                type="button"
                title="Selected text captured from the other app. Click to discard."
                onClick={() => setSelection(null)}
                className="shrink-0 rounded-full bg-accent/15 px-2.5 py-0.5 text-xs text-accent"
              >
                {selection.length} chars selected ✕
              </button>
            )}
          </div>
          <CommandList items={items} selected={selected} onHover={setSelected} onPick={(c) => start(c, selection, slashArgs)} />
          <Footer
            status={notice ?? (slashArgs ? `Text: ${slashArgs.slice(0, 60)}` : selection ? undefined : platform?.auto_capture === false ? "Selection capture unavailable" : undefined)}
            hints={[
              { keys: "↑↓", label: "Navigate" },
              { keys: "↵", label: "Run", onClick: () => items[selected] && start(items[selected], selection, slashArgs) },
              ...(query.trimStart().startsWith("/") ? [{ keys: "Tab", label: "Complete" }] : []),
              { keys: "Ctrl H", label: "History", onClick: () => void openHistory() },
              { keys: "Esc", label: "Close", onClick: () => void close() },
            ]}
          />
        </>
      )}

      {view.name === "chat" && (
        <ChatView
          messages={chatMessages}
          busy={chatBusy}
          error={chatError}
          progress={progress}
          context={chatContext}
          notice={notice}
          onSend={sendChat}
          onRetry={retryChat}
          onNewChat={newChat}
          onDropContext={() => setChatContext(null)}
          onCopy={(t) => void chatCopy(t)}
          onPaste={(t) => void historyPaste(t)}
          onBack={leaveChat}
        />
      )}

      {view.name === "history" && (
        <HistoryView
          entries={history}
          notice={notice}
          onCopy={(t) => void historyCopy(t)}
          onPaste={(t) => void historyPaste(t)}
          onDelete={(id) => void historyDelete(id)}
          onBack={backToSearch}
        />
      )}

      {view.name === "input" && (
        <InputView
          cmd={view.cmd}
          initial={view.initial}
          onBack={backToSearch}
          onSubmit={(text) => void run(view.cmd, text, selection !== null && text === selection)}
        />
      )}

      {view.name === "run" && (
        <ResultView
          cmd={view.cmd}
          state={view.state}
          progress={progress}
          canReplace={canReplace && view.state.kind === "text"}
          platform={platform}
          notice={notice}
          onCopy={() => void copy()}
          onRetry={retry}
          onReplace={() => void replace()}
          onEditInput={editInput}
          onOpenSettings={() => void api.openSettings()}
          onBack={backToSearch}
        />
      )}
    </div>
  );
}
