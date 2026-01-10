## 2024-05-22 - Optimizing string-to-bytes stream conversion
**Learning:** In Rust, `[String.clone().as_bytes(), b"\n"].concat()` involves multiple allocations and copies. When the original string has excess capacity (which is common when building strings with `String::with_capacity`), using `push('\n')` followed by `into_bytes()` is significantly faster because it often avoids reallocation and allows zero-copy conversion to `Bytes`.
**Action:** When streaming strings that need a suffix, prefer mutating the string in place if ownership allows, rather than cloning and concatenating.
