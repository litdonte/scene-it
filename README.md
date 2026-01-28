# Scene It

**Scene It** is a scene-driven writing application designed to help storytellers create, experiment with, and refine narratives in a flexible, visual way. Whether you’re crafting a screenplay, novel, or interactive story, Scene It provides a modern desktop experience backed by a Rust core and AI-assisted guidance.

Built with **Tauri**, Scene It combines a native desktop UI with a high-performance Rust domain engine.

---

## Overview

At its core, **Scene It** treats a story as a **linear path of scenes** within a **web of scene drafts**. This enables writers to explore multiple creative directions without losing track of earlier ideas.

- **Scene Variants:** Swap out drafts of a scene to explore alternate outcomes.
- **AI-Generated Templates:** Start with a summary and instantly generate a workable scene structure.
- **Story Flow Optimization:** Unsure which scene fits best? AI suggests ordering and pacing.
- **Script Grading:** Compare your script’s structure against award-winning works.

The application uses a Rust-based story engine for modeling scenes, characters, and relationships, with a Tauri frontend providing a responsive desktop UI.

---

## Features

### Scene Management

- Create, edit, and organize scenes in a flexible graph-based structure.
- Add multiple **variants** of the same scene.
- Instantly switch between drafts to test pacing or character arcs.

### AI-Assisted Writing

- Generate scene templates from short summaries.
- Receive suggestions for improving consistency, clarity, and narrative coherence.
- Detect deviations from character motivations or established arcs.

### Script Evaluation

- Grade scripts against professional benchmarks.
- Get insights on pacing, structure, and character development.
- Compare alternate versions for narrative impact.

### Character Development

- Define character backstories, motivations, arcs, and relationships.
- Receive feedback when actions or dialogue break characterization.
- Maintain continuity across variants and rewrites.

### Desktop Application (Tauri)

- Cross-platform native desktop app powered by Tauri.
- Rust core for story modeling and mutation.
- Modern GUI for browsing storyboards, editing scenes, and managing characters.
- Foundation for future visual graph exploration and collaboration tools.

---

## How It Works

1. **Create a Scene** — Start from scratch or from a short summary.
2. **Generate Variants** — Explore alternate drafts without losing your original.
3. **Refine with AI** — Tighten dialogue, pacing, structure, and character voice.
4. **Analyze Story Flow** — Understand which scene orders best support your intent.
5. **Grade Your Script** — Benchmark your writing against industry standards.

---

## Use Cases

- Screenwriters experimenting with dialogue, structure, or scene order.
- Novelists exploring branching plotlines or alternate character decisions.
- Storytellers using AI to ensure consistency and narrative strength.
- Writers preparing work for contests, feedback groups, or professional submission.

---

## Roadmap & Milestones

### **v0.1 — Core Data Model (Complete)**

- [x] Scene, SceneVariant, and Character type definitions  
- [x] Storyboard core model  
- [x] Generic ID types for all domain objects  
- [x] Metadata system for timestamps and revision history  

### **v0.2 — Scene Graph & Story Flow (In Progress)**

- [x] Directed graph for scene ordering  
- [x] Tools for reordering scenes  
- [ ] Visual graph exploration (GUI)

### **v0.3 — Desktop UI (Tauri)**

- [ ] Storyboard browser  
- [ ] Scene editor  
- [ ] Character manager  
- [ ] Variant switching UI  
- [ ] Live script preview  

### **v1.0 — AI-Assisted Writing Tools**

- [ ] AI-generated scene templates  
- [ ] Dialogue tone analysis  
- [ ] Character consistency evaluations  
- [ ] Scene flow optimization suggestions  
- [ ] Full AI grading system (structure, pacing, character development)  

### **v1.1 — Exporting & Sharing Tools**

- [ ] PDF/Markdown/Final Draft export  
- [ ] Script formatting by template type  
- [ ] Version comparison viewer  

### **Future / Experimental**

- Terminal UI (Ratatui) as an optional alternative interface
- Branching narrative support (choose-your-own-adventure style)
- Collaboration mode
- Cloud syncing (optional)
- Plugin system for community extensions

---

## Contributing

Contributions are welcome!  
Please open an issue or submit a pull request. Feature ideas, bug fixes, and UX improvements are appreciated.

---

## License

This project is licensed under the **MIT License**.
