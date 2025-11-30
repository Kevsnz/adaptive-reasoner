pub(crate) const API_URL: &str = "http://192.173.1.67:1234/v1";

pub(crate) static MODEL_MAPPING: phf::Map<&'static str, (&str, i32)> = phf::phf_map! {
    "qwen3-4b-low" => ("qwen/qwen3-4b-thinking-2507",128),
    "qwen3-4b-medium" => ("qwen/qwen3-4b-thinking-2507",512),
    "qwen3-4b-high" => ("qwen/qwen3-4b-thinking-2507",4096),
    "qwen3-30b-low" => ("qwen3-30b-a3b-thinking-2507",128),
    "qwen3-30b-medium" => ("qwen3-30b-a3b-thinking-2507",512),
    "qwen3-30b-high" => ("qwen3-30b-a3b-thinking-2507",2048),
};
