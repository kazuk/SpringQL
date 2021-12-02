// Copyright (c) 2021 TOYOTA MOTOR CORPORATION. Licensed under MIT OR Apache-2.0.

pub(crate) mod naive_row_repository;

pub(crate) use naive_row_repository::NaiveRowRepository;

use super::Row;
use crate::{error::Result, stream_engine::autonomous_executor::task::task_id::TaskId};
use std::fmt::Debug;

/// # Concept diagram
///
/// `[1]` represents a stream. `--a-->` represents a task.
///
/// ```text
/// ---a---->[1]---b------------>[2]---d-------------->[4]---f---------->
///           |    in buf: []          in buf: []       ^    in buf: []
///           |                                         |
///           +----c------------>[3]---e----------------+
///                in buf: []          in buf: []
/// ```
///
/// ```text
/// emit(r1, vec!["b", "c"]);
///
/// ---a---->[1]---b------------>[2]---d-------------->[4]---f---------->
///           |    in buf: [r1]        in buf: []       ^    in buf: []
///           |                                         |
///           +----c------------>[3]---e----------------+
///                in buf: [r1]        in buf: []
/// ```
///
/// ```text
/// collect_next("b");  // -> r1
///
/// ---a---->[1]---b------------>[2]---d-------------->[4]---f---------->
///           |    in buf: []          in buf: []       ^    in buf: []
///           |                                         |
///           +----c------------>[3]---e----------------+
///                in buf: [r1]        in buf: []
/// ```
///
/// ```text
/// emit(r2, vec!["b", "c"]);
///
/// ---a---->[1]---b------------>[2]---d-------------->[4]---f---------->
///           |    in buf: [r2]        in buf: []       ^    in buf: []
///           |                                         |
///           +----c------------>[3]---e----------------+
///                in buf: [r2,r1]     in buf: []
/// ```
///
/// ```text
/// collect_next("c");  // -> r1
///
/// ---a---->[1]---b------------>[2]---d-------------->[4]---f---------->
///           |    in buf: [r2]        in buf: []       ^    in buf: []
///           |                                         |
///           +----c------------>[3]---e----------------+
///                in buf: [r2]        in buf: []
/// ```
///
/// ```text
/// emit(r3, "f");
///
/// ---a---->[1]---b------------>[2]---d-------------->[4]---f---------->
///           |    in buf: [r2]        in buf: []       ^    in buf: [r3]
///           |                                         |
///           +----c------------>[3]---e----------------+
///                in buf: [r2]        in buf: []
/// ```
pub(crate) trait RowRepository: Debug + Default + Sync + Send {
    /// Get the next row as `task`'s input.
    ///
    /// # Failure
    ///
    /// - [SpringError::InputTimeout](crate::error::SpringError::InputTimeout) when:
    ///   - next row is not available within `timeout`
    fn collect_next(&self, task: &TaskId) -> Result<Row> {
        log::debug!("[RowRepository] collect_next({:?})", task);
        self._collect_next(task)
    }
    fn _collect_next(&self, task: &TaskId) -> Result<Row>;

    /// Move `row` to downstream tasks.
    fn emit(&self, row: Row, downstream_tasks: &[TaskId]) -> Result<()> {
        debug_assert!(!downstream_tasks.is_empty());
        log::debug!(
            "[RowRepository] emit_owned({:?}, {:?})",
            row,
            downstream_tasks
        );
        self._emit(row, downstream_tasks)
    }
    fn _emit(&self, row: Row, downstream_tasks: &[TaskId]) -> Result<()>;

    /// Reset repository with new tasks.
    fn reset(&self, tasks: Vec<TaskId>) {
        log::debug!("[RowRepository] reset({:?})", tasks);
        self._reset(tasks)
    }
    fn _reset(&self, tasks: Vec<TaskId>);
}
