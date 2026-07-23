## Projects as Folders: Edit Anywhere, Stay Canonical

A project folder mirrors one thread's active source onto disk. Edit `model.ecky` with any file-based tool; Ecky validates the changed file and records accepted updates as new thread versions. The thread remains canonical history.

`project_folder_export` writes two files:

```text
<projectsRoot>/<slug>/
  model.ecky          edit this with anything
  ecky-project.json   binding manifest, owned by Ecky — never edit by hand
```

Edit `model.ecky` in any editor. A polling watcher detects a digest change, compiles the source, renders a preview, and commits a `folder-sync` version on the bound thread. Two safeguards prevent partial or repeated failures:

- **Two-tick settle.** A changed file must read identical on two consecutive polls before the compiler sees it. A half-written save — the editor flushing in chunks — never reaches Ecky mid-write.
- **A broken save fails once, loudly, then waits.** If the edited source does not compile, the watcher reports the failure once for that exact content and then goes quiet until you change the file again. It does not re-render the same mistake every tick.

When you need to reason about the folder explicitly, `project_folder_status` classifies it:

- `clean` — file matches the bound version; nothing to do.
- `fileChanged` — you edited the file; the watcher will apply it (or you can).
- `threadAdvanced` — the thread moved on without the folder; the folder is stale. Re-export to refresh it.
- `conflict` — both sides moved. The watcher will **not** auto-resolve this; applying requires an explicit force, and the previous head stays available as a version so nothing is lost.
- `missing` — no folder or no manifest yet.

**The folder is a mirror, not a second database.** Threads and versions remain authoritative. Do not edit `ecky-project.json`; refresh a stale mirror or resolve a conflict explicitly.
