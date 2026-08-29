# Codex Provider Integration Delta

## MODIFIED Requirements

### Requirement: Core persistence is provider-neutral

Bindings, lineage, normalized finished messages, and FIFO SHALL namespace external
ids by adapter. One Ecky thread MAY retain one current binding for each configured
provider so mode switching never overwrites another provider's durable conversation.

#### Scenario: Another adapter is added

- **WHEN** Antigravity creates a conversation for an Ecky thread already bound to Codex
- **THEN** both bindings coexist under their provider ids
- **AND** each adapter reuses shared Ecky handoff and FIFO semantics
