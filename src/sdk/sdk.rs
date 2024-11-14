use pyo3::prelude::*;
use pyo3::exceptions::PyModuleNotFoundError;
use pyo3::types::{PyDict, PyList};
use std::sync::Once;
use thiserror::Error;

static INIT: Once = Once::new();

#[derive(Debug, Clone)]
pub struct PragmaPrice {
    pub pair: String,
    pub price: u64,
    pub timestamp: u64,
    pub source: String,
    pub publisher: String,
    pub volume: u64,
}

#[derive(Debug, Error)]
pub enum PragmaSDKError {
    #[error("Python error: {0}")]
    PythonError(#[from] PyErr),
    #[error("Failed to fetch price data: {0}")]
    FetchError(String),
}

fn init_python() -> PyResult<()> {
    Python::with_gil(|py| {
        if let Err(e) = py.import_bound("pragma_sdk") {
            return Err(PyModuleNotFoundError::new_err(e));
        }

        let setup_code = r#"
import asyncio
from pragma_sdk.common.fetchers.fetcher_client import FetcherClient
from pragma_sdk.common.fetchers.fetchers import BitstampFetcher
from pragma_sdk.common.fetchers.fetchers.gateio import GateioFetcher
from pragma_sdk.common.types.pair import Pair

async def fetch_prices(pair_strings):
    pairs = [Pair.from_tickers(*pair_str.split('/')) for pair_str in pair_strings]
    
    bitstamp_fetcher = BitstampFetcher(pairs, "pragma_miden")
    gateio_fetcher = GateioFetcher(pairs, "pragma_miden")
    fetchers = [bitstamp_fetcher, gateio_fetcher]
    
    fc = FetcherClient()
    fc.add_fetchers(fetchers)
    
    return await fc.fetch()
        "#;
        py.run_bound(setup_code, None, None)
    })
}

pub async fn get_pragma_prices(pairs: Vec<String>) -> Result<Vec<PragmaPrice>, PragmaSDKError> {
    INIT.call_once(|| {
        init_python().expect("Failed to initialize Python runtime");
    });

    Python::with_gil(|py| {
        let pairs_list = PyList::new_bound(py, pairs.iter());
        let locals = PyDict::new_bound(py);
        locals.set_item("pairs", pairs_list)?;

        py.run_bound(
            "result = asyncio.run(fetch_prices(pairs))",
            None,
            Some(&locals),
        )?;

        let result = locals
            .get_item("result")?
            .ok_or(PragmaSDKError::FetchError("No result returned".into()))?;

        let mut prices = Vec::new();
        for entry in result.iter()? {
            let entry = entry?;
            prices.push(PragmaPrice {
                pair: entry.getattr("pair_id")?.extract()?,
                price: entry.getattr("price")?.extract()?,
                timestamp: entry.getattr("timestamp")?.extract()?,
                source: entry.getattr("source")?.extract()?,
                publisher: entry.getattr("publisher")?.extract()?,
                volume: entry.getattr("volume")?.extract()?,
            });
        }

        Ok(prices)
    })
}
