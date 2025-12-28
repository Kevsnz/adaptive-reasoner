use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ObjectType {
    Model,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    AdaptiveReasoner,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelList {
    pub data: Vec<Model>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Model {
    pub id: String,
    pub object: ObjectType,
    pub created: i64,
    pub owned_by: Owner,
}
