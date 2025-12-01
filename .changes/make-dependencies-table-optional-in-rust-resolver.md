---
semifold-resolver: "patch:fix"
---

Rust projects may not have a [dependencies] section (e.g., pure library crates or those with only dev-dependencies). This change makes the dependencies table optional instead of requiring it.
