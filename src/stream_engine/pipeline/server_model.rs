pub(crate) mod server_type;

use serde::{Deserialize, Serialize};

use crate::model::option::Options;

use self::server_type::ServerType;

/// See: <https://docs.sqlstream.com/sql-reference-guide/create-statements/createserver/#prebuilt-server-objects-available-in-sserver>
#[derive(Debug, Serialize, Deserialize, new)]
pub(crate) struct ServerModel {
    server_type: ServerType,
    options: Options,
}