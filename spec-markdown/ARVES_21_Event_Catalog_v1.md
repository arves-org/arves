> **Rendered from `ARVES_21_Event_Catalog_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES-21: Event Catalog v1.0

STATUS: EVENT CONSTITUTION (AUTHORITATIVE EVENT INVENTORY)

# Purpose

Define all canonical events, event contracts, ownership boundaries and communication standards across the ARVES platform.

# Platform Events

• tenant.created

• workspace.created

• user.invited

• policy.updated

• notification.sent

# Information Events

• provider.registered

• data.discovered

• entity.resolved

• knowledge.created

• knowledge.updated

• ontology.updated

# Cognitive Events

• context.created

• memory.created

• memory.updated

• reasoning.started

• reasoning.completed

• decision.created

• reflection.completed

# Strategic Events

• goal.created

• goal.updated

• plan.generated

• strategy.selected

• priority.changed

# Experience Events

• conversation.started

• conversation.completed

• search.executed

• presence.changed

• dashboard.updated

# Evolution Events

• learning.completed

• benchmark.completed

• preference.updated

• calibration.completed

# Embodied Events

• vision.detected

• location.updated

• navigation.started

• physical.action.executed

# Agent Events

• agent.created

• agent.activated

• agent.goal.assigned

• agent.task.delegated

• agent.completed

# Runtime Events

• event.received

• workflow.started

• task.started

• task.completed

• task.failed

• schedule.triggered

# Canonical Event Envelope

event_id, event_type, tenant_id, workspace_id, correlation_id, timestamp, source, payload, version

# Event Contract Template

Event Name, Owner Service, Producer, Consumers, Payload Schema, Version, Retry Policy, Idempotency Rules, Security Classification.

# Event Governance

All events must be versioned, traceable, auditable and backward compatible where possible.

# Final Definition

Event Catalog = Authoritative Inventory of All ARVES Event Contracts.
