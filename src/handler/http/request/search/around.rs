// Copyright 2025 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::io::{Error, ErrorKind};

use actix_web::{HttpResponse, http::StatusCode, web};
use chrono::{Duration, Utc};
use config::{
    DEFAULT_SEARCH_AROUND_FIELDS, TIMESTAMP_COL_NAME,
    meta::{
        search::SearchEventType,
        self_reporting::usage::{RequestStats, UsageType},
        stream::StreamType,
    },
    metrics,
    utils::{
        base64,
        json::{self, get_string_value},
    },
};
use hashbrown::HashMap;
use infra::errors;
use tracing::{Instrument, Span};

use crate::{
    common::{
        meta,
        utils::http::{get_stream_type_from_request, get_work_group},
    },
    service::{
        search as SearchService,
        self_reporting::{http_report_metrics, report_request_usage_stats},
    },
};

pub(crate) async fn around(
    trace_id: String,
    http_span: Span,
    org_id: String,
    stream_name: String,
    query: web::Query<HashMap<String, String>>,
    body: Option<web::Bytes>,
    user_id: Option<String>,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();

    let stream_type = get_stream_type_from_request(&query).unwrap_or_default();

    let mut around_key = match query.get("key") {
        Some(v) => v.parse::<i64>().unwrap_or(0),
        None => 0,
    };
    let mut query_fn = query
        .get("query_fn")
        .and_then(|v| base64::decode_url(v).ok());
    if let Some(vrl_function) = &query_fn {
        if !vrl_function.trim().ends_with('.') {
            query_fn = Some(format!("{} \n .", vrl_function));
        }
    }

    let default_sql = format!("SELECT * FROM \"{}\" ", stream_name);
    let mut around_sql = match query.get("sql") {
        None => default_sql,
        Some(v) => match base64::decode_url(v) {
            Err(_) => default_sql,
            Ok(sql) => sql,
        },
    };

    // check playload
    let mut filters = HashMap::new();
    if let Some(body) = body {
        let data: json::Value = json::from_slice(&body).unwrap_or_default();
        if let Some(data) = data.as_object() {
            if let Some(key) = data.get(TIMESTAMP_COL_NAME) {
                if let Some(ts) = key.as_i64() {
                    around_key = ts;
                }
            }
            for field in DEFAULT_SEARCH_AROUND_FIELDS.iter() {
                if let Some(value) = data.get(field) {
                    if value.is_null() {
                        continue;
                    }
                    filters.insert(field.to_string(), get_string_value(value));
                }
            }
        }
    }
    if !filters.is_empty() {
        around_sql = SearchService::sql::add_new_filters_with_and_operator(&around_sql, filters)
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    }

    let around_size = query
        .get("size")
        .map_or(10, |v| v.parse::<i64>().unwrap_or(10));

    let regions = query.get("regions").map_or(vec![], |regions| {
        regions
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    let clusters = query.get("clusters").map_or(vec![], |clusters| {
        clusters
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });

    metrics::QUERY_PENDING_NUMS
        .with_label_values(&[&org_id])
        .inc();
    // get a local search queue lock
    #[cfg(not(feature = "enterprise"))]
    let locker = SearchService::QUEUE_LOCKER.clone();
    #[cfg(not(feature = "enterprise"))]
    let locker = locker.lock().await;
    #[cfg(not(feature = "enterprise"))]
    if !config::get_config().common.feature_query_queue_enabled {
        drop(locker);
    }
    #[cfg(not(feature = "enterprise"))]
    let took_wait = start.elapsed().as_millis() as usize;
    #[cfg(feature = "enterprise")]
    let took_wait = 0;
    log::info!(
        "http search around API wait in queue took: {} ms",
        took_wait
    );
    metrics::QUERY_PENDING_NUMS
        .with_label_values(&[&org_id])
        .dec();

    let timeout = query
        .get("timeout")
        .map_or(0, |v| v.parse::<i64>().unwrap_or(0));
    let around_start_time = around_key
        - Duration::try_seconds(900)
            .unwrap()
            .num_microseconds()
            .unwrap();
    let around_end_time = around_key
        + Duration::try_seconds(900)
            .unwrap()
            .num_microseconds()
            .unwrap();

    // search forward
    let fw_sql = SearchService::sql::check_or_add_order_by_timestamp(&around_sql, false)
        .unwrap_or(around_sql.to_string());
    let req = config::meta::search::Request {
        query: config::meta::search::Query {
            sql: fw_sql,
            from: 0,
            size: around_size / 2,
            start_time: around_start_time,
            end_time: around_key,
            quick_mode: false,
            query_type: "".to_string(),
            track_total_hits: false,
            uses_zo_fn: false,
            query_fn: query_fn.clone(),
            action_id: None,
            skip_wal: false,
            streaming_output: false,
            streaming_id: None,
        },
        encoding: config::meta::search::RequestEncoding::Empty,
        regions: regions.clone(),
        clusters: clusters.clone(),
        timeout,
        search_type: Some(SearchEventType::UI),
        search_event_context: None,
        use_cache: None,
    };
    let search_res = SearchService::search(&trace_id, &org_id, stream_type, user_id.clone(), &req)
        .instrument(http_span.clone())
        .await;

    let resp_forward = match search_res {
        Ok(res) => res,
        Err(err) => {
            http_report_metrics(start, &org_id, stream_type, "500", "_around");
            log::error!("search around error: {:?}", err);
            return Ok(match err {
                errors::Error::ErrorCode(code) => match code {
                    errors::ErrorCodes::SearchCancelQuery(_) => HttpResponse::TooManyRequests()
                        .json(meta::http::HttpResponse::error_code_with_trace_id(
                            code,
                            Some(trace_id),
                        )),
                    _ => HttpResponse::InternalServerError().json(
                        meta::http::HttpResponse::error_code_with_trace_id(code, Some(trace_id)),
                    ),
                },
                _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    err.to_string(),
                )),
            });
        }
    };

    // search backward
    let bw_sql = SearchService::sql::check_or_add_order_by_timestamp(&around_sql, true)
        .unwrap_or(around_sql.to_string());
    let req = config::meta::search::Request {
        query: config::meta::search::Query {
            sql: bw_sql,
            from: 0,
            size: around_size / 2,
            start_time: around_key,
            end_time: around_end_time,
            quick_mode: false,
            query_type: "".to_string(),
            track_total_hits: false,
            uses_zo_fn: false,
            query_fn: query_fn.clone(),
            action_id: None,
            skip_wal: false,
            streaming_output: false,
            streaming_id: None,
        },
        encoding: config::meta::search::RequestEncoding::Empty,
        regions,
        clusters,
        timeout,
        search_type: Some(SearchEventType::UI),
        search_event_context: None,
        use_cache: None,
    };
    let search_res = SearchService::search(&trace_id, &org_id, stream_type, user_id.clone(), &req)
        .instrument(http_span)
        .await;

    let resp_backward = match search_res {
        Ok(res) => res,
        Err(err) => {
            http_report_metrics(start, &org_id, stream_type, "500", "_around");
            log::error!("search around error: {:?}", err);
            return Ok(match err {
                errors::Error::ErrorCode(code) => match code {
                    errors::ErrorCodes::SearchCancelQuery(_) => HttpResponse::TooManyRequests()
                        .json(meta::http::HttpResponse::error_code_with_trace_id(
                            code,
                            Some(trace_id),
                        )),
                    _ => HttpResponse::InternalServerError().json(
                        meta::http::HttpResponse::error_code_with_trace_id(code, Some(trace_id)),
                    ),
                },
                _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    err.to_string(),
                )),
            });
        }
    };

    // merge
    let mut resp = config::meta::search::Response::default();
    let hits_num = resp_backward.hits.len();
    for i in 0..hits_num {
        resp.hits
            .push(resp_backward.hits[hits_num - 1 - i].to_owned());
    }
    let hits_num = resp_forward.hits.len();
    for i in 0..hits_num {
        resp.hits.push(resp_forward.hits[i].to_owned());
    }
    resp.total = resp.hits.len();
    resp.size = around_size;
    resp.scan_size = resp_forward.scan_size + resp_backward.scan_size;
    resp.took = resp_forward.took + resp_backward.took;
    resp.cached_ratio = (resp_forward.cached_ratio + resp_backward.cached_ratio) / 2;

    let time = start.elapsed().as_secs_f64();
    http_report_metrics(start, &org_id, stream_type, "200", "_around");

    let req_stats = RequestStats {
        records: resp.hits.len() as i64,
        response_time: time,
        size: resp.scan_size as f64,
        request_body: Some(req.query.sql),
        user_email: user_id,
        min_ts: Some(around_start_time),
        max_ts: Some(around_end_time),
        cached_ratio: Some(resp.cached_ratio),
        trace_id: Some(trace_id),
        took_wait_in_queue: match (
            resp_forward.took_detail.as_ref(),
            resp_backward.took_detail.as_ref(),
        ) {
            (Some(forward_took), Some(backward_took)) => {
                Some(forward_took.cluster_wait_queue + backward_took.cluster_wait_queue)
            }
            (Some(forward_took), None) => Some(forward_took.cluster_wait_queue),
            (None, Some(backward_took)) => Some(backward_took.cluster_wait_queue),
            _ => None,
        },
        work_group: get_work_group(vec![
            resp_forward.work_group.clone(),
            resp_backward.work_group.clone(),
        ]),
        ..Default::default()
    };
    let num_fn = req.query.query_fn.is_some() as u16;
    report_request_usage_stats(
        req_stats,
        &org_id,
        &stream_name,
        StreamType::Logs,
        UsageType::SearchAround,
        num_fn,
        started_at,
    )
    .await;

    Ok(HttpResponse::Ok().json(resp))
}
