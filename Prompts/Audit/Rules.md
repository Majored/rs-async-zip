# Audit

## Instructions
You are auditing an asynchronous ZIP archive reading/writing Rust crate against the following rules.
If the user has not specified what you're auditing against (eg. unstaged changes, staged commits, a specific branch vs main, etc), please ask them.
Do not tell the user what is compliant - only flag all violations of these rules.

## Rules
### Eveywhere
- The lib.rs module docs must match the README.md, but should excluding the usage section and onwards.
- Use terminology consistent with the rest of the create (see [`crate::base`] documentation).
    - For instance, do not confuse a local file (LF) with a local file header (LFH).
- Use already established abbreviations for common ZIP concepts and structures.
- Avoid typos.
- All .rs files must use the standardised copyright header.
- Code comments should only be used to document non-obvious invariants.
- If the crate version is being bumped, it must also be bumped in the README and lib.rs module documentation.

### Read-only
- When seeking to an offset, validate the offset is within the reader's length via [`crate::base::read1::validate_offset()`].
- Use the CDR values over the LF values when we have access to the CDR.