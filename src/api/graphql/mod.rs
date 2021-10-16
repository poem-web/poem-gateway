mod input;
mod mutation;
mod query;

use std::sync::Arc;

use anyhow::Result;
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, Schema,
};
use async_graphql_poem::GraphQL;
use poem::{endpoint::make_sync, get, web::Html, Endpoint, Response, Route};
use serde::{Deserialize, Serialize};

use crate::config::EndpointConfig;

#[derive(Serialize, Deserialize, Clone)]
struct Config {}

#[typetag::serde(name = "graphqlAdmin")]
impl EndpointConfig for Config {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>> {
        let route = Route::new()
            .at(
                "/playground",
                get(make_sync(|req| {
                    let uri = req.original_uri();
                    Html(playground_source(GraphQLPlaygroundConfig::new(&format!(
                        "{}/query",
                        uri.path().strip_suffix("/playground").unwrap()
                    ))))
                })),
            )
            .at(
                "query",
                GraphQL::new(Schema::new(
                    query::Query,
                    mutation::Mutation,
                    EmptySubscription,
                )),
            );

        Ok(Arc::new(route))
    }
}
