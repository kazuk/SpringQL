use std::rc::Rc;

use crate::stream_engine::autonomous_executor::data::row::Row;

#[derive(PartialEq, Debug, new)]
pub(in crate::stream_engine::autonomous_executor::exec) enum FinalRow {
    /// The same row as query plan input.
    Preserved(Rc<Row>),

    /// Newly created row during query plan execution.
    NewlyCreated(Row),
}