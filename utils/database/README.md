# Embedded database
The goal here is to explore exposing a SQL interface to plugins in the global context.

This can be useful for:
1. Passive inter-plugin communication eg another plugin adding a task(task plugin would display it). 
2. Markdown processing. A data processor that targets specific markdown tags.

## Issues
- Requires more work for permissions.

## Acknowledgments:
[baildon](https://github.com/garypen/baildon/): a library which implements a simple B+Tree
