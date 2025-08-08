// use eyre::Result;
// use reqwest;
// use serde::{Deserialize, Serialize};

// const LIQD_API_URL: &str = "https://api.liqd.ag";

// #[derive(Debug, Clone)]
// pub struct Liqd {
//     api_url: String,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct Allocation {
//     pub routerIndex: u8,
//     pub percentage: u32,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct Route {
//     pub tokenIn: String,
//     pub tokenOut: String,
//     pub allocations: Vec<Allocation>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct RouteResponseData {
//     pub path: Vec<String>,
//     pub hop: Vec<Allocation>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct RouteResponse {
//     pub success: bool,
//     pub route: Route,
// }

// impl Liqd {
//     pub fn new() -> Self {
//         Self { api_url: LIQD_API_URL.to_string() }
//     }

//     pub async fn _get_route(
//         &self,
//         from: &str,
//         to: &str,
//         exact_in: bool,
//         amount: f32,
//     ) -> Result<RouteResponse> {
//         let client = reqwest::Client::new();

//         let query = if exact_in {
//             format!("?tokenA={}&tokenB={}&amountIn={}&multihop=true", from, to, amount)
//         } else {
//             format!("?tokenA={}&tokenB={}&amountOut={}&multihop=true", from, to, amount)
//         };

//         let response = client.get(&format!("{}/route{}", self.api_url, query)).send().await?;

//         // Check if the response status is successful
//         if !response.status().is_success() {
//             return Err(eyre::eyre!(
//                 "API request failed with status: {} - {}",
//                 response.status(),
//                 response
//                     .text()
//                     .await
//                     .unwrap_or_else(|_| "Unable to read error response".to_string())
//             ));
//         }

//         let route_response: RouteResponse = response.json().await?;
//         Ok(route_response)
//     }
// }
