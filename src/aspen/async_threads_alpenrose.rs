use catenary::aspen::lib::*;
use catenary::aspen_dataset::GtfsRtType;
use catenary::postgres_tools::CatenaryPostgresPool;
use crossbeam::deque::{Injector, Steal};
use gtfs_realtime::FeedMessage;
use scc::HashMap as SccHashMap;
use std::collections::HashSet;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

use crate::import_alpenrose::new_rt_data;

pub async fn alpenrose_process_threads(
    alpenrose_to_process_queue: Arc<Injector<ProcessAlpenroseData>>,
    authoritative_gtfs_rt_store: Arc<SccHashMap<(String, GtfsRtType), FeedMessage>>,
    authoritative_data_store: Arc<SccHashMap<String, catenary::aspen_dataset::AspenisedData>>,
    conn_pool: Arc<CatenaryPostgresPool>,
    alpenrosethreadcount: usize,
    chateau_queue_list: Arc<Mutex<HashSet<String>>>,
    lease_id_for_this_worker: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut set: JoinSet<_> = (0usize..alpenrosethreadcount)
        .map(|i| {
            let alpenrose_to_process_queue = Arc::clone(&alpenrose_to_process_queue);
            let authoritative_gtfs_rt_store = Arc::clone(&authoritative_gtfs_rt_store);
            let authoritative_data_store = Arc::clone(&authoritative_data_store);
            let conn_pool = Arc::clone(&conn_pool);
            let chateau_queue_list = Arc::clone(&chateau_queue_list);
            async move {
                alpenrose_loop_process_thread(
                    alpenrose_to_process_queue,
                    authoritative_gtfs_rt_store,
                    authoritative_data_store,
                    conn_pool,
                    chateau_queue_list,
                )
                .await
            }
        })
        .collect();

    while let Some(res) = set.join_next().await {
        res.unwrap().unwrap();
    }

    Ok(())
}

pub async fn alpenrose_loop_process_thread(
    alpenrose_to_process_queue: Arc<Injector<ProcessAlpenroseData>>,
    authoritative_gtfs_rt_store: Arc<SccHashMap<(String, GtfsRtType), FeedMessage>>,
    authoritative_data_store: Arc<SccHashMap<String, catenary::aspen_dataset::AspenisedData>>,
    conn_pool: Arc<CatenaryPostgresPool>,
    chateau_queue_list: Arc<Mutex<HashSet<String>>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        // println!("From-Alpenrose process thread");
        match alpenrose_to_process_queue.steal() {
            Steal::Success(new_ingest_task) => {
                let feed_id = new_ingest_task.realtime_feed_id.clone();

                let mut chateau_queue_list = chateau_queue_list.lock().await;

                chateau_queue_list.remove(&new_ingest_task.chateau_id.clone());

                drop(chateau_queue_list);

                let rt_processed_status = new_rt_data(
                    Arc::clone(&authoritative_data_store),
                    Arc::clone(&authoritative_gtfs_rt_store),
                    new_ingest_task.chateau_id,
                    new_ingest_task.realtime_feed_id,
                    new_ingest_task.has_vehicles,
                    new_ingest_task.has_trips,
                    new_ingest_task.has_alerts,
                    new_ingest_task.vehicles_response_code,
                    new_ingest_task.trips_response_code,
                    new_ingest_task.alerts_response_code,
                    Arc::clone(&conn_pool),
                )
                .await;

                if let Err(e) = &rt_processed_status {
                    eprintln!("Error processing RT data: {} {:?}", feed_id, e);
                }
            }
            _ => {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
    }

    Ok(())
}
