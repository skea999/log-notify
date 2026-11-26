//! inotify tailer.
//!
//! Tracks offsets, partial lines, the line cap, shrink resets, and inode
//! replacement. Every source starts at end of file.
