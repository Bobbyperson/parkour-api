use std::collections::HashMap;
use warp::{Filter, Rejection, Reply, http::StatusCode};

use crate::{Store, slug::slugify};
use serde::{Deserialize, Serialize};

pub type MapRoutes = HashMap<String, Vec<MapRoute>>;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Line {
    origin: [f64; 3],
    angles: [i64; 3],
    dimensions: [i64; 2],
    trigger: [[f64; 3]; 2],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RouteName {
    origin: [f64; 3],
    angles: [i64; 3],
    dimensions: [i64; 2],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LeaderboardSource {
    origin: [f64; 3],
    angles: [i64; 3],
    dimensions: [i64; 2],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Leaderboard {
    origin: [f64; 3],
    angles: [i64; 3],
    dimensions: [i64; 2],
    source: LeaderboardSource,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Leaderboards {
    local: Leaderboard,
    world: Leaderboard,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct StartPosition {
    origin: [f64; 3],
    angles: [i64; 3],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct EndPosition {
    origin: [f64; 3],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Robot {
    origin: [f64; 3],
    angles: [i64; 3],
    talkable_radius: i64,
    animation: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct StartIndicator {
    coordinates: [f64; 3],
    trigger_radius: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MapObject {
    coordinates: [f64; 3],
    angles: [f64; 3],
    scale: f64,
    model_name: String,
    hidden: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MapRoute {
    pub name: String,
    start_line: Line,
    finish_line: Line,
    leaderboards: Leaderboards,
    checkpoints: Vec<[f64; 3]>,
    start: StartPosition,
    end: EndPosition,
    ziplines: Vec<[[f64; 3]; 2]>,
    perks: Option<HashMap<String, String>>,
    robot: Robot,
    indicator: StartIndicator,
    route_name: RouteName,
    entities: Option<Vec<MapObject>>,
}

pub fn post_json() -> impl Filter<Extract = (MapRoute,), Error = Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

async fn create_map_route(
    map_name: String,
    mut entry: MapRoute,
    store: Store,
) -> Result<impl Reply, Rejection> {
    let routes_list = store.routes_list.read();
    let map_routes = match routes_list.get(&map_name) {
        Some(r) => r,
        None => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&"Map not found."),
                StatusCode::NOT_FOUND,
            ));
        }
    };

    let slug = slugify(&entry.name);
    let exists = map_routes.iter().any(|r| slugify(&r.name) == slug);
    if exists {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Route name already used."),
            StatusCode::ALREADY_REPORTED,
        ));
    }

    if entry.perks.is_none() {
        entry.perks = Some(HashMap::new());
    }
    if entry.entities.is_none() {
        entry.entities = Some(Vec::new());
    }

    drop(routes_list);
    store
        .routes_list
        .write()
        .get_mut(&map_name)
        .unwrap()
        .push(entry);

    Ok(warp::reply::with_status(
        warp::reply::json(&slug),
        StatusCode::CREATED,
    ))
}

async fn get_map_routes(map_name: String, store: Store) -> Result<impl Reply, Rejection> {
    let routes_read_lock = store.routes_list.read();
    match routes_read_lock.get(&map_name) {
        None => Ok(warp::reply::with_status(
            warp::reply::json(&"Map not found."),
            StatusCode::NOT_FOUND,
        )),
        Some(routes) => {
            let map: HashMap<String, serde_json::Value> = routes
                .iter()
                .map(|r| (slugify(&r.name), serde_json::to_value(r).unwrap()))
                .collect();
            Ok(warp::reply::with_status(
                warp::reply::json(&map),
                StatusCode::OK,
            ))
        }
    }
}

pub fn get_routes(store: Store) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let store_filter = warp::any().map(move || store.clone());

    let create = warp::post()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::param())
        .and(warp::path("routes"))
        .and(warp::path::end())
        .and(post_json())
        .and(store_filter.clone())
        .and_then(create_map_route);

    let list = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::param())
        .and(warp::path("routes"))
        .and(warp::path::end())
        .and(store_filter)
        .and_then(get_map_routes);

    create.or(list)
}
