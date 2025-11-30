use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ObjectType {
    Model,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Owner {
    AdaptiveReasoner,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct ModelList {
    pub(crate) data: Vec<Model>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Model {
    pub(crate) id: String,
    pub(crate) object: ObjectType,
    pub(crate) created: i64,
    pub(crate) owned_by: Owner,
}
