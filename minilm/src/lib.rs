use std::{borrow::Borrow, str::FromStr, sync::Arc};

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;

#[derive(Clone)]
pub struct MiniLM {
    device: Device,
    model: Arc<BertModel>,
    tokenizer: Tokenizer,
}

const MINILM_DIM: usize = 384;

/// A 384 dimensional embedding
#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MiniLmEmbedding(Vec<f32>);

#[non_exhaustive]
pub struct EmbeddedStr<'a> {
    pub text: &'a str,
    pub embedding: MiniLmEmbedding,
}

impl<'a> EmbeddedStr<'a> {
    fn to_owned(&self) -> EmbeddedString {
        EmbeddedString {
            text: self.text.to_string(),
            embedding: self.embedding.clone(),
        }
    }
}

#[non_exhaustive]
pub struct EmbeddedString {
    pub text: String,
    pub embedding: MiniLmEmbedding,
}

impl MiniLmEmbedding {
    pub fn new(data: Vec<f32>) -> Option<Self> {
        (data.len() == MINILM_DIM).then_some(MiniLmEmbedding(data))
    }

    pub fn as_slice(&self) -> &[f32] {
        self.0.as_slice()
    }
}

const BERT_CONFIG: &str = include_str!("../data/config.json");

const TOKENIZER_JSON: &str = include_str!("../data/tokenizer.json");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to construct model: {0}")]
    CandleError(#[from] candle_core::Error),
    #[error("Failed to tokenize {0}")]
    TokenizerError(#[from] tokenizers::Error),
    #[error("Model output was incorrect dimension")]
    ModelOutputError,
}

type Result<T> = std::result::Result<T, Error>;

impl MiniLM {
    pub fn build(bytes: Vec<u8>) -> Result<Self> {
        let device = Device::Cpu;
        let builder =
            VarBuilder::from_buffered_safetensors(bytes, candle_core::DType::F32, &device)?;

        let config = serde_json::from_str::<Config>(BERT_CONFIG)
            .expect("Failed to parse bert config even though it is embedded in binary");
        let model = Arc::new(BertModel::load(builder, &config)?);
        let tkzr = tokenizers::Tokenizer::from_str(TOKENIZER_JSON)
            .expect("Failed to parse tokenizer even though it is embedded in binary");
        Ok(MiniLM {
            device,
            model,
            tokenizer: tkzr,
        })
    }

    pub fn embed_string(&self, text: String) -> Result<EmbeddedString> {
        let tokens = self.tokenizer.encode(text.as_str(), true)?;
        let input_ids = Tensor::from_slice(tokens.get_ids(), (tokens.len(),), &self.device)?;
        let type_ids = Tensor::from_slice(tokens.get_type_ids(), (tokens.len(),), &self.device)?;

        let embedding =
            MiniLmEmbedding::new(self.model.forward(&input_ids, &type_ids, None)?.to_vec1()?)
                .ok_or(Error::ModelOutputError)?;
        Ok(EmbeddedString { text, embedding })
    }

    pub fn embed_str<'a>(&self, text: &'a str) -> Result<EmbeddedStr<'a>> {
        let tokens = self.tokenizer.encode(text, true)?;
        let input_ids = Tensor::from_slice(tokens.get_ids(), (tokens.len(),), &self.device)?;
        let type_ids = Tensor::from_slice(tokens.get_type_ids(), (tokens.len(),), &self.device)?;

        let embedding =
            MiniLmEmbedding::new(self.model.forward(&input_ids, &type_ids, None)?.to_vec1()?)
                .ok_or(Error::ModelOutputError)?;
        Ok(EmbeddedStr { text, embedding })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;
    use candle_nn::VarBuilder;
    use candle_transformers::models::bert::{self, Config};

    #[test]
    fn it_works() {
        let device = Device::Cpu;
        let st = std::fs::read("./data/minilm.safetensors").expect("failed to open weights");
        let builder = VarBuilder::from_buffered_safetensors(st, candle_core::DType::F32, &device)
            .expect("Failed to load");
        let config = std::fs::read_to_string("./data/config.json").expect("failed to open config");
        let config = serde_json::from_str::<Config>(&config).expect("Failed to parse config");
        let model = bert::BertModel::load(builder, &config).expect("Failed to load model");
        let tkzr = tokenizers::Tokenizer::from_file("data/tokenizer.json")
            .expect("failed to load tokenizer");
        let sample_input = "Hellow world";
        tkzr.encode_fast(sample_input, true)
            .expect("Failed to encode text");
    }
}
