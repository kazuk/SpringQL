// Copyright (c) 2021 TOYOTA MOTOR CORPORATION. Licensed under MIT OR Apache-2.0.

pub(self) mod task_worker_thread_handler;

mod generic_worker_pool;
mod scheduler;
mod source_worker_pool;
mod task_executor_lock;

use crate::{error::Result, low_level_rs::SpringConfig};
use std::sync::Arc;

use self::{
    generic_worker_pool::GenericWorkerPool,
    source_worker_pool::SourceWorkerPool,
    task_executor_lock::{PipelineUpdateLockGuard, TaskExecutorLock},
};
use super::{
    event_queue::EventQueue, pipeline_derivatives::PipelineDerivatives, repositories::Repositories,
    task_graph::TaskGraph, worker::worker_handle::WorkerStopCoordinate,
};

/// Task executor executes task graph's dataflow by internal worker threads.
/// Source tasks are scheduled by SourceScheduler and other tasks are scheduled by FlowEfficientScheduler (in Moderate state) or MemoryReducingScheduler (in Severe state).
///
/// All interface methods are called from main thread, while `new()` spawns worker threads.
#[derive(Debug)]
pub(in crate::stream_engine) struct TaskExecutor {
    task_executor_lock: Arc<TaskExecutorLock>,

    repos: Arc<Repositories>,

    generic_worker_pool: GenericWorkerPool,
    source_worker_pool: SourceWorkerPool,
}

impl TaskExecutor {
    pub(in crate::stream_engine::autonomous_executor) fn new(
        config: &SpringConfig,
        event_queue: Arc<EventQueue>,
        worker_stop_coordinate: Arc<WorkerStopCoordinate>,
    ) -> Self {
        let task_executor_lock = Arc::new(TaskExecutorLock::default());
        let repos = Arc::new(Repositories::new(config));

        Self {
            task_executor_lock: task_executor_lock.clone(),

            generic_worker_pool: GenericWorkerPool::new(
                config.worker.n_generic_worker_threads,
                event_queue.clone(),
                worker_stop_coordinate.clone(),
                task_executor_lock.clone(),
                repos.clone(),
            ),
            source_worker_pool: SourceWorkerPool::new(
                config.worker.n_source_worker_threads,
                event_queue,
                worker_stop_coordinate,
                task_executor_lock,
                repos.clone(),
            ),

            repos,
        }
    }

    /// AutonomousExecutor acquires lock when pipeline is updated.
    pub(in crate::stream_engine::autonomous_executor) fn pipeline_update_lock(
        &self,
    ) -> PipelineUpdateLockGuard {
        self.task_executor_lock.pipeline_update()
    }

    /// Update workers' internal current pipeline.
    pub(in crate::stream_engine::autonomous_executor) fn update_pipeline(
        &self,
        _lock_guard: &PipelineUpdateLockGuard,
        pipeline_derivatives: Arc<PipelineDerivatives>,
    ) -> Result<()> {
        let pipeline = pipeline_derivatives.pipeline();
        pipeline
            .all_sources()
            .into_iter()
            .try_for_each(|source_reader| {
                self.repos
                    .source_reader_repository()
                    .register(source_reader)
            })?;
        pipeline
            .all_sinks()
            .into_iter()
            .try_for_each(|sink_writer| {
                self.repos.sink_writer_repository().register(sink_writer)
            })?;

        Ok(())
    }

    /// Stop all source tasks and executes pump tasks and sink tasks to finish all rows remaining in queues.
    pub(in crate::stream_engine::autonomous_executor) fn cleanup(
        &self,
        _lock_guard: &PipelineUpdateLockGuard,
        task_graph: &TaskGraph,
    ) {
        // TODO do not just remove rows in queues. Do the things in doc comment.

        self.repos
            .row_queue_repository()
            .reset(task_graph.row_queues().into_iter().collect());
        self.repos
            .window_queue_repository()
            .reset(task_graph.window_queues().into_iter().collect());
    }
}
