// Copyright 2022 Zinc Labs Inc. and Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::StreamType;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Dashboards {
    pub dashboards: Vec<Dashboard>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    #[serde(default)]
    pub dashboard_id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default = "datetime_now")]
    #[schema(value_type = String, format = DateTime)]
    pub created: DateTime<FixedOffset>,
    #[serde(default)]
    pub panels: Vec<Panel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layouts: Option<Vec<Layout>>,
    pub variables: Option<Variables>,
}

fn datetime_now() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).expect(
        "BUG", // This can't possibly fail. Can it?
    ))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub i: i64,
    pub panel_id: String,
    #[serde(rename = "static")]
    pub is_static: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub id: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub fields: PanelFields,
    pub config: PanelConfig,
    pub query: String,
    #[serde(default)]
    pub query_type: String,
    pub custom_query: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PanelFields {
    pub stream: String,
    pub stream_type: StreamType,
    pub x: Vec<AxisItem>,
    pub y: Vec<AxisItem>,
    pub filter: Vec<PanelFilter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AxisItem {
    pub label: String,
    pub alias: String,
    pub column: String,
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregation_function: Option<AggregationFunc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AggregationFunc {
    Count,
    #[serde(rename = "count-distinct")]
    CountDistinct,
    Histogram,
    Sum,
    Min,
    Max,
    Avg,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PanelFilter {
    #[serde(rename = "type")]
    pub typ: String,
    pub values: Vec<String>,
    pub column: String,
    pub operator: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PanelConfig {
    title: String,
    description: String,
    show_legends: bool,
    legends_position: Option<String>,
    promql_legend: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variables {
    pub list: Vec<List>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    #[serde(rename = "type")]
    pub type_field: String,
    pub name: String,
    pub label: String,
    #[serde(rename = "query_data")]
    pub query_data: Option<QueryData>,
    pub value: Option<String>,
    pub options: Option<Vec<CustomFieldsOption>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryData {
    #[serde(rename = "stream_type")]
    pub stream_type: StreamType,
    pub stream: String,
    pub field: String,
    pub max_record_size: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldsOption {
    pub label: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::json;
    use expect_test::expect;

    #[test]
    fn test_de1() {
        let dashboard: Dashboard = json::from_str(r##"{
            "title": "b2",
            "dashboardId": "1501078512",
            "description": "desc2",
            "role": "",
            "owner": "root@example.com",
            "created": "2023-03-30T07:49:41.744+00:00",
            "panels": [
                {
                "id": "Panel_ID7857010",
                "type": "bar",
                "fields": {
                    "stream": "default",
                    "stream_type": "logs",
                    "x": [
                    {
                        "label": "Timestamp",
                        "alias": "x_axis_1",
                        "column": "_timestamp",
                        "color": null,
                        "aggregationFunction": "histogram"
                    }
                    ],
                    "y": [
                    {
                        "label": "Kubernetes Host",
                        "alias": "y_axis_1",
                        "column": "kubernetes_host",
                        "color": "#5960b2",
                        "aggregationFunction": "count"
                    }
                    ],
                    "filter": [
                    {
                        "type": "condition",
                        "values": [],
                        "column": "method",
                        "operator": "Is Not Null",
                        "value": null
                    }
                    ]
                },
                "config": {
                    "title": "p5",
                    "description": "sample config blah blah blah",
                    "show_legends": true,
                    "legends_position": "bottom",
                    "promql_legend": "right"
                },
                "query": "SELECT histogram(_timestamp) as \"x_axis_1\", count(kubernetes_host) as \"y_axis_1\"  FROM \"default\" WHERE method IS NOT NULL GROUP BY \"x_axis_1\" ORDER BY \"x_axis_1\"",
                "customQuery": false
                }
            ],
            "layouts": [
                {
                "x": 0,
                "y": 0,
                "w": 12,
                "h": 13,
                "i": 1,
                "panelId": "Panel_ID7857010",
                "static": false
                }
            ]
        }"##).unwrap();

        expect![[r##"
            Dashboard {
                dashboard_id: "1501078512",
                title: "b2",
                description: "desc2",
                role: "",
                owner: "root@example.com",
                created: 2023-03-30T07:49:41.744+00:00,
                panels: [
                    Panel {
                        id: "Panel_ID7857010",
                        typ: "bar",
                        fields: PanelFields {
                            stream: "default",
                            stream_type: Logs,
                            x: [
                                AxisItem {
                                    label: "Timestamp",
                                    alias: "x_axis_1",
                                    column: "_timestamp",
                                    color: None,
                                    aggregation_function: Some(
                                        Histogram,
                                    ),
                                },
                            ],
                            y: [
                                AxisItem {
                                    label: "Kubernetes Host",
                                    alias: "y_axis_1",
                                    column: "kubernetes_host",
                                    color: Some(
                                        "#5960b2",
                                    ),
                                    aggregation_function: Some(
                                        Count,
                                    ),
                                },
                            ],
                            filter: [
                                PanelFilter {
                                    typ: "condition",
                                    values: [],
                                    column: "method",
                                    operator: Some(
                                        "Is Not Null",
                                    ),
                                    value: None,
                                },
                            ],
                        },
                        config: PanelConfig {
                            title: "p5",
                            description: "sample config blah blah blah",
                            show_legends: true,
                            legends_position: Some(
                                "bottom",
                            ),
                            promql_legend: Some(
                                "right",
                            ),
                        },
                        query: "SELECT histogram(_timestamp) as \"x_axis_1\", count(kubernetes_host) as \"y_axis_1\"  FROM \"default\" WHERE method IS NOT NULL GROUP BY \"x_axis_1\" ORDER BY \"x_axis_1\"",
                        query_type: "",
                        custom_query: false,
                    },
                ],
                layouts: Some(
                    [
                        Layout {
                            x: 0,
                            y: 0,
                            w: 12,
                            h: 13,
                            i: 1,
                            panel_id: "Panel_ID7857010",
                            is_static: false,
                        },
                    ],
                ),
                variables: None,
            }
        "##]].assert_debug_eq(&dashboard);
    }

    #[test]
    fn test_de2() {
        let dashboard: Dashboard = json::from_str(r##"{
            "dashboardId": "7049428968893710336",
            "title": "board1",
            "description": "",
            "role": "",
            "owner": "root@example.com",
            "created": "2023-04-05T17:13:58.204+00:00",
            "panels": [
              {
                "id": "Panel_ID1135310",
                "type": "bar",
                "fields": {
                  "stream": "default",
                  "stream_type": "logs",
                  "x": [
                    {
                      "label": "Timestamp",
                      "alias": "x_axis_1",
                      "column": "_timestamp",
                      "color": null,
                      "aggregationFunction": "histogram"
                    }
                  ],
                  "y": [
                    {
                      "label": "Kubernetes Host",
                      "alias": "y_axis_1",
                      "column": "kubernetes_host",
                      "color": "#5960b2",
                      "aggregationFunction": "count"
                    }
                  ],
                  "filter": [
                    {
                      "type": "condition",
                      "values": [],
                      "column": "log",
                      "operator": "Is Not Null",
                      "value": null
                    },
                    {
                      "type": "list",
                      "values": [
                        "stdout",
                        "stderr"
                      ],
                      "column": "stream",
                      "operator": null,
                      "value": null
                    }
                  ]
                },
                "config": {
                  "title": "p1",
                  "description": "",
                  "show_legends": true
                },
                "query": "SELECT histogram(_timestamp) as \"x_axis_1\", count(kubernetes_host) as \"y_axis_1\"  FROM \"default\" WHERE log IS NOT NULL AND stream IN ('stdout', 'stderr') GROUP BY \"x_axis_1\" ORDER BY \"x_axis_1\"",
                "query_type": "",
                "customQuery": false
              }
            ],
            "layouts": [
              {
                "x": 0,
                "y": 0,
                "w": 12,
                "h": 13,
                "i": 1,
                "panelId": "Panel_ID1135310",
                "static": false
              }
            ]
        }"##).unwrap();

        expect![[r##"
            Dashboard {
                dashboard_id: "7049428968893710336",
                title: "board1",
                description: "",
                role: "",
                owner: "root@example.com",
                created: 2023-04-05T17:13:58.204+00:00,
                panels: [
                    Panel {
                        id: "Panel_ID1135310",
                        typ: "bar",
                        fields: PanelFields {
                            stream: "default",
                            stream_type: Logs,
                            x: [
                                AxisItem {
                                    label: "Timestamp",
                                    alias: "x_axis_1",
                                    column: "_timestamp",
                                    color: None,
                                    aggregation_function: Some(
                                        Histogram,
                                    ),
                                },
                            ],
                            y: [
                                AxisItem {
                                    label: "Kubernetes Host",
                                    alias: "y_axis_1",
                                    column: "kubernetes_host",
                                    color: Some(
                                        "#5960b2",
                                    ),
                                    aggregation_function: Some(
                                        Count,
                                    ),
                                },
                            ],
                            filter: [
                                PanelFilter {
                                    typ: "condition",
                                    values: [],
                                    column: "log",
                                    operator: Some(
                                        "Is Not Null",
                                    ),
                                    value: None,
                                },
                                PanelFilter {
                                    typ: "list",
                                    values: [
                                        "stdout",
                                        "stderr",
                                    ],
                                    column: "stream",
                                    operator: None,
                                    value: None,
                                },
                            ],
                        },
                        config: PanelConfig {
                            title: "p1",
                            description: "",
                            show_legends: true,
                            legends_position: None,
                            promql_legend: None,
                        },
                        query: "SELECT histogram(_timestamp) as \"x_axis_1\", count(kubernetes_host) as \"y_axis_1\"  FROM \"default\" WHERE log IS NOT NULL AND stream IN ('stdout', 'stderr') GROUP BY \"x_axis_1\" ORDER BY \"x_axis_1\"",
                        query_type: "",
                        custom_query: false,
                    },
                ],
                layouts: Some(
                    [
                        Layout {
                            x: 0,
                            y: 0,
                            w: 12,
                            h: 13,
                            i: 1,
                            panel_id: "Panel_ID1135310",
                            is_static: false,
                        },
                    ],
                ),
                variables: None,
            }
        "##]].assert_debug_eq(&dashboard);
    }
}
