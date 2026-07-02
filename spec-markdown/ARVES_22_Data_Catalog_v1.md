> **Rendered from `ARVES_22_Data_Catalog_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES-22: Data Catalog v1.0

STATUS: DATA CONSTITUTION (AUTHORITATIVE DATA INVENTORY)

# Purpose

Define all canonical data domains, ownership boundaries, storage systems, governance rules and lifecycle management across ARVES.

# Identity Data

• Users

• Roles

• Permissions

• Policies

• Memberships

# Tenant Data

• Tenants

• Workspaces

• Organizations

• Teams

# Knowledge Data

• Knowledge Objects

• Facts

• Insights

• Evidence

• Ontology Definitions

# Graph Data

• Entities

• Relationships

• Knowledge Graph Edges

# Cognitive Data

• Contexts

• Memories

• Reasoning Artifacts

• Decisions

# Strategic Data

• Goals

• Plans

• Strategies

• Simulations

• Priorities

# Experience Data

• Conversations

• Messages

• Search History

• Notifications

# Evolution Data

• Learning Records

• Benchmarks

• Preferences

• Reflections

# Agent Data

• Agents

• Capabilities

• Delegations

• Execution History

# Runtime Data

• Events

• Tasks

• Workflows

• Schedules

• Execution State

# Embodied Data

• Sensor Data

• Maps

• Locations

• World State

# Storage Architecture

PostgreSQL (transactional data), Graph Database (knowledge graph), Vector Store (embeddings and semantic memory), Object Storage (documents/files), Redis (cache and runtime state).

# Data Ownership Model

Every dataset has a single owning service and owning core. Data ownership is tenant-aware and policy-governed.

# Data Lifecycle

Created → Validated → Stored → Used → Updated → Archived → Deleted.

# Data Governance

Classification, lineage, retention, auditability, provenance and compliance controls apply to all data assets.

# Canonical Data Contract Template

Data Name, Owner, Storage, Schema, Classification, Lifecycle, APIs, Events, Retention Policy.

# Final Definition

Data Catalog = Authoritative Inventory of All ARVES Data Assets.
