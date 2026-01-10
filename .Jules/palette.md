## 2024-05-22 - Icon Button Accessibility
**Learning:** `IconButton` components were completely inaccessible to screen readers as they lacked `aria-label` or `title` attributes, relying solely on visual icons.
**Action:** Added `aria_label` and `title` props to `IconButton` with a fallback to `name`. Updated usages to provide descriptive labels. In future, ensure all interactive elements have accessible names.
