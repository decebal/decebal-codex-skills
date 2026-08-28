---
title: Keep Documentation Accessible and Searchable
impact: LOW
impactDescription: reduces information retrieval time by 70%
tags: doc, accessibility, search, findability
---

## Keep Documentation Accessible and Searchable

Store documentation where the team can find it. Inaccessible docs are useless docs. Optimize for discoverability, not organization aesthetics.

**Incorrect (documentation scattered or hidden):**

```markdown
## Where Documentation Lives

- PRD v1: Alice's Google Drive
- PRD v2: Shared folder (which one?)
- Technical notes: Bob's personal Notion
- API docs: README in repo (which branch?)
- Meeting notes: Various Slack threads
- Decisions: "I think we discussed this in email"

// Finding information requires asking around
// New team members can't self-serve
// Knowledge leaves when people leave
```

**Correct (centralized and searchable):**

````markdown
## Documentation Accessibility Standards

### Central Hub

All project documentation accessible from one entry point:

```text
📁 Project Dashboard - Documentation Hub
│
├── 📄 README (Start Here)
│   └── Links to all key documents
│
├── 📁 Product
│   ├── PRD (current version)
│   ├── User Research
│   └── Roadmap
│
├── 📁 Technical
│   ├── Architecture Overview
│   ├── API Documentation
│   └── ADRs (Decision Records)
│
├── 📁 Design
│   ├── Design System
│   └── Figma Links
│
├── 📁 Process
│   ├── Meeting Notes
│   ├── Retrospectives
│   └── Launch Checklists
│
└── 📁 Archive
    └── Superseded documents
```

### Searchability Requirements

**Text-based formats:**
- Use Markdown, not PDFs
- Plain text over images of text
- Headings for structure (enables TOC)
- Keywords in document body

**Naming conventions:**
```yaml
# Good - searchable, sortable
2024-03-15-prd-user-dashboard-v3.md
adr-005-database-selection.md
meeting-2024-03-15-sprint-planning.md

# Bad - unclear, unsearchable
doc.md
final_FINAL_v2.docx
notes.txt
```

**Tagging and metadata:**
```yaml
---
title: User Dashboard PRD
type: prd
project: dashboard
status: approved
created: 2024-02-01
updated: 2024-03-15
owner: carol.davis
tags:
  - dashboard
  - analytics
  - q1-2024
---
```

### Access Control

| Document Type | Access Level | Reason |
|---------------|--------------|--------|
| PRD | All employees | Transparency |
| Technical Spec | Engineering + Product | Technical detail |
| Salary data | HR + Exec | Confidential |
| Security audit | Security team | Sensitive |

**Default:** Public within organization unless specifically restricted.

### Discoverability Aids

**README in every folder:**
```markdown
# Technical Documentation

This folder contains technical specifications for Project Dashboard.

## Quick Links
- Architecture Overview: `./architecture.md` - System design
- API Reference: `./api/` - Endpoint documentation
- ADRs: `./adrs/` - Why we made key decisions

## Can't Find Something?
- Search the wiki for keywords
- Ask in #proj-dashboard Slack
- Check the Archive folder
```

**Glossary for project-specific terms:**
```markdown
## Glossary

| Term | Definition |
|------|------------|
| Dashboard | The analytics display feature |
| Widget | Individual component on dashboard |
| KPI | Key Performance Indicator |
| DAU | Daily Active Users |
```

### Maintenance

**Monthly documentation review:**
- [ ] Update broken links
- [ ] Archive obsolete docs
- [ ] Verify access permissions
- [ ] Check search indexing
- [ ] Update README quick links

**Documentation owner responsibilities:**
- Keep assigned docs current
- Respond to questions within 24h
- Mark deprecated docs clearly
- Redirect to current versions
````

**Accessibility requirements:**
- Single entry point for all docs
- Text-based searchable formats
- Consistent naming conventions
- Appropriate access controls
- Regular maintenance schedule

Reference: [Write the Docs - Documentation Principles](https://www.writethedocs.org/guide/)
