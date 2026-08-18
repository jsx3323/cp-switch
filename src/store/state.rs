use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// cp-switch 的单一状态文件结构。
///
/// 用 BTreeMap 而非 HashMap：serde 按迭代序输出，HashMap 的迭代序每进程随机，
/// 会让 state.json 的 key 顺序每次写盘都变。
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct State {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub profiles: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub project_currents: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_current: Option<String>,
}
