# Store calls Studio stands in for

The contract of `knowqore-stores` lists the Lessons only as one stream
(`Lessons`), in the store's order, with no count, sort, window, or filter.
`src-tauri/src/lessons.rs` stands in for the calls below: it reads every
Lesson once per open store, keeps a small row per Lesson, and answers the
frontend's commands (`lessons_page`, `lesson_tree`) from that buffer. The
frontend already speaks the finished API, so taking the real calls changes
only that module (owner, 23 October 2026).

## Wanted from the store

1. **A window of Lessons**: a sort (statement, kind, status, transfer,
   updated), an offset or a cursor, a limit, and the total, answered from
   the store's index without reading every record.
2. **A filter by container**: the Lessons seen in one container, or under a
   prefix of container ids (`github.com/sonalect`), with the same sort and
   window, narrowed also by kind, status, transfer, and polarity (several
   values of one field are "or", fields are "and"), with the count of
   Lessons per value of each of the four fields under the same container
   (`lesson_facets`), and grouped by the next level of container ids below the
   prefix (the groups with their counts and the window across them, folded
   groups listing only their header). Today Studio places a Lesson by `Support.origins`, which names at
   most five containers, so a Lesson known in more is found under its first
   five only. `LessonContainersOf` has them all, but one call per Lesson.
3. **The containers with their counts**: each container with its kind and
   the number of distinct Lessons seen in it, so the tree needs no pass over
   the Lessons.

**Why:** a Vault already holds about 1,500 Lessons and grows; the buffer
reads all of them before the first row shows.

**How to apply:** agree the request, reply, and error codes with the owner
and make the change in Knowqore first (`.claude/rules/upstream.md`); then
move the `rev` and replace the buffer in `lessons.rs`, keeping the commands'
wire form.
