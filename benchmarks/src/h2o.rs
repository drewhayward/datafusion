// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::path::{Path, PathBuf};

use crate::{BenchmarkRun, CommonOpt};
use datafusion::{error::Result, prelude::SessionContext};
use datafusion_common::{exec_datafusion_err, instant::Instant, DataFusionError};
use structopt::StructOpt;

/// Run the H2o.ai benchmark
///
/// [1]: https://github.com/h2oai/db-benchmark
#[derive(Debug, StructOpt, Clone)]
#[structopt(verbatim_doc_comment)]
pub struct RunOpt {
    /// Query number (between 1 and 10). If not specified, runs all queries
    #[structopt(short, long)]
    query: Option<usize>,

    /// Common options
    #[structopt(flatten)]
    common: CommonOpt,

    /// Path to queries.sql (single file)
    #[structopt(
        parse(from_os_str),
        short = "r",
        long = "queries-path",
        default_value = "benchmarks/queries/h2o/groupby.sql"
    )]
    queries_path: PathBuf,

    /// Path to group by csv data
    #[structopt(
        parse(from_os_str),
        short = "p",
        long = "path",
        default_value = "benchmarks/data/G1_1e7_1e2_0_0.csv"
    )]
    path: PathBuf,

    /// If present, write results json here
    #[structopt(parse(from_os_str), short = "o", long = "output")]
    output_path: Option<PathBuf>,
}

impl RunOpt {
    pub async fn run(self) -> Result<()> {
        println!("Running benchmarks with the following options: {self:?}");
        let queries = AllQueries::try_new(&self.queries_path)?;
        let query_range = match self.query {
            Some(query_id) => query_id..=query_id,
            None => queries.min_query_id()..=queries.max_query_id(),
        };

        let config = self.common.config();
        let ctx = SessionContext::new_with_config(config);

        // Register data
        self.register_data(&ctx).await?;

        let iterations = self.common.iterations;
        let mut benchmark_run = BenchmarkRun::new();
        for query_id in query_range {
            benchmark_run.start_new_case(&format!("Query {query_id}"));
            let sql = queries.get_query(query_id)?;
            println!("Q{query_id}: {sql}");

            for i in 0..iterations {
                let start = Instant::now();
                let results = ctx.sql(sql).await?.collect().await?;
                let elapsed = start.elapsed();
                let ms = elapsed.as_secs_f64() * 1000.0;
                let row_count: usize = results.iter().map(|b| b.num_rows()).sum();
                println!(
                    "Query {query_id} iteration {i} took {ms:.1} ms and returned {row_count} rows"
                );
                benchmark_run.write_iter(elapsed, row_count);
            }
        }

        Ok(())
    }

    async fn register_data(&self, ctx: &SessionContext) -> Result<()> {
        let options = Default::default();
        let path = self.path.as_os_str().to_str().unwrap();

        ctx.register_csv("x", path, options).await.map_err(|e| {
            DataFusionError::Context(
                format!("Registering 'table' as {path}"),
                Box::new(e),
            )
        })
    }
}

struct AllQueries {
    queries: Vec<String>,
}

impl AllQueries {
    fn try_new(path: &Path) -> Result<Self> {
        let all_queries = std::fs::read_to_string(path)
            .map_err(|e| exec_datafusion_err!("Could not open {path:?}: {e}"))?;

        Ok(Self {
            queries: all_queries.lines().map(|s| s.to_string()).collect(),
        })
    }

    fn get_query(&self, query_id: usize) -> Result<&str> {
        self.queries
            .get(query_id + 1)
            .ok_or_else(|| {
                let min_id = self.min_query_id();
                let max_id = self.max_query_id();
                exec_datafusion_err!(
                    "Invalid query id {query_id}. Must be between {min_id} and {max_id}"
                )
            })
            .map(|s| s.as_str())
    }

    fn min_query_id(&self) -> usize {
        1
    }

    fn max_query_id(&self) -> usize {
        self.queries.len()
    }
}

#[derive(Debug, StructOpt)]
struct GroupBy {
    /// Query number
    #[structopt(short, long)]
    query: usize,
    /// Path to data file
    #[structopt(parse(from_os_str), required = true, short = "p", long = "path")]
    path: PathBuf,
    /// Activate debug mode to see query results
    #[structopt(short, long)]
    debug: bool,
    /// Load the data into a MemTable before executing the query
    #[structopt(short = "m", long = "mem-table")]
    mem_table: bool,
    /// Path to machine readable output file
    #[structopt(parse(from_os_str), short = "o", long = "output")]
    output_path: Option<PathBuf>,
}