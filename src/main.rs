use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Datelike, Local, Timelike};
use serde::{Deserialize, Serialize};
use tokio;

#[derive(Debug, Deserialize)]
struct PriceData {
    #[serde(rename = "NOK_per_kWh")]
    nok_kwh: f64,
    #[serde(rename = "EUR_per_kWh")]
    eur_kwh: f64,
    #[serde(rename = "time_start")]
    time_start: String,
}

#[derive(Debug, Serialize)]
struct ChartDataPoint {
    hour: u32,
    price: f64,
    time: String,
    price_nok: f64,
    price_eur: f64,
}

async fn fetch() -> Result<Vec<PriceData>, Box<dyn std::error::Error>> {
    let now = Local::now();
    let url = format!(
        "https://www.hvakosterstrommen.no/api/v1/prices/{}/{:02}-{:02}_NO2.json",
        now.year(),
        now.month(),
        now.day()
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let data: Vec<PriceData> = response.json().await?;
    Ok(data)
}

async fn prices() -> impl IntoResponse {
    match fetch().await {
        Ok(data) => {
            let chart: Vec<ChartDataPoint> = data
                .into_iter()
                .map(|item| {
                    let hour = if let Ok(dt) = DateTime::parse_from_rfc3339(&item.time_start) {
                        dt.hour()
                    } else {
                        0
                    };

                    ChartDataPoint {
                        hour,
                        price: item.nok_kwh * 100.0,

                        time: item.time_start,
                        price_nok: item.nok_kwh,
                        price_eur: item.eur_kwh,
                    }
                })
                .collect();

            Json(chart).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>elektron</title>
    <style>
        @font-face {
            font-family: 'JetBrainsMono';
            src: url('./src/font/Regular.woff2') format('woff2');
            font-weight: 400;
        }
        @font-face {
            font-family: 'JetBrainsMono';
            src: url('./src/font/Bold.woff2') format('woff2');
            font-weight: 700;
        }

        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: 'JetBrains Mono', monospace;
            background: white;
            color: black;
            padding: 20px;
            line-height: 1.4;
        }

        #date {
            margin-bottom: 10px;
            font-weight: bold;
        }

        #data {
            white-space: pre;
            margin-top: 20px;
        }

        #graph {
            margin-top: 30px;
            width: 100%;
        }

        .error {
            border: 1px solid black;
            padding: 10px;
            margin: 10px 0;
        }

        #tooltip {
            position: absolute;
            background: black;
            color: white;
            padding: 5px;
            font-family: 'JetBrains Mono', monospace;
            font-size: 12px;
            display: none;
            pointer-events: none;
            z-index: 1000;
        }
    </style>
</head>
<body>
    <div id="date"></div>

    <div id="error" class="error" style="display: none;"></div>
    <div id="data"></div>
    <div id="graph"></div>
    <div id="tooltip"></div>

    <script>
        let chartData = null;

        async function loadData() {
            const error = document.getElementById('error');
            const data = document.getElementById('data');

            error.style.display = 'none';
            data.innerHTML = '';

            try {
                const response = await fetch('/prices');
                if (!response.ok) {
                    throw new Error('Feilkode ' + response.status + ' :-(');
                }

                const priceData = await response.json();

                if (priceData.length === 0) {
                    throw new Error('A-hva? Jeg fant ikke noe data :-(');
                }

                displayData(priceData);

                chartData = priceData;
                createGraph(priceData);

            } catch (err) {
                error.textContent = err.message;
                error.style.display = 'block';
            }
        }

        function displayData(priceData) {
            const prices = priceData.map(item => item.price);
            const avgPrice = prices.reduce((a, b) => a + b, 0) / prices.length;
            const maxPrice = Math.max(...prices);
            const minPrice = Math.min(...prices);

            // Display current date
            const now = new Date();
            const dateStr = now.getDate().toString().padStart(2, '0') + '-' +
                           (now.getMonth() + 1).toString().padStart(2, '0') + '-' +
                           now.getFullYear();
            document.getElementById('date').textContent = dateStr;

            let output = '';

            // Prices
            priceData.forEach(item => {
                const price = item.price.toFixed(1).padStart(5, ' ');
                output += price + '\t';
            });
            output += '\n';

            // Hours
            priceData.forEach(item => {
                const date = new Date(item.time);
                const hour = date.getHours().toString().padStart(2, '0').padStart(5, ' ');
                output += hour + '\t';
            });
            output += '\n\n';

            // Statistics
            output += 'Maksverdi    ' + maxPrice.toFixed(1).padStart(5, ' ') + '\n';
            output += 'Gjennomsnitt ' + avgPrice.toFixed(1).padStart(5, ' ') + '\n';
            output += 'Minsteverdi  ' + minPrice.toFixed(1).padStart(5, ' ');

            document.getElementById('data').textContent = output;
        }

        function createGraph(priceData) {
            const container = document.getElementById('graph');
            const width = container.offsetWidth || 800;
            const height = 200;
            const margin = { top: 20, right: 20, bottom: 20, left: 60 };
            const graphWidth = width - margin.left - margin.right;
            const graphHeight = height - margin.top - margin.bottom;

            const prices = priceData.map(d => d.price);
            const minPrice = Math.min(...prices);
            const maxPrice = Math.max(...prices);

            const elements = prices.length;
            console.log(elements);

            let svg = '<svg width="' + width + '" height="' + height + '" style="background: white;">';

            // Y-axis tick marks and labels
            for (let i = 0; i <= 4; i++) {
                const value = minPrice + (maxPrice - minPrice) * i / 4;
                const y = height - margin.bottom - (graphHeight * i / 4);
                svg += '<text x="' + (margin.left - 10) + '" y="' + (y + 4) + '" text-anchor="end" font-size="10" fill="black">' + value.toFixed(1) + '</text>';
            }

            // X-axis tick marks and labels
            for (let i = 0; i < elements; i += 1) {
                const x = margin.left + (graphWidth * i / (elements - 1));
                svg += '<text x="' + x + '" y="' + (height - 5) + '" text-anchor="middle" font-size="10" fill="black">' + i.toString().padStart(2, '0') + '</text>';
            }

            // Step line path
            let pathData = '';
            priceData.forEach((item, index) => {
                const x = margin.left + (graphWidth * item.hour / 23);
                const y = height - margin.bottom - ((item.price - minPrice) / (maxPrice - minPrice)) * graphHeight;

                if (index === 0) {
                    pathData += 'M' + x + ',' + y;
                } else {
                    const prevItem = priceData[index - 1];
                    const prevX = margin.left + (graphWidth * prevItem.hour / 23);
                    pathData += 'L' + prevX + ',' + y + 'L' + x + ',' + y;
                }
            });

            svg += '<path d="' + pathData + '" stroke="black" stroke-width="1" fill="none"/>';

            // Invisible hover rectangles
            priceData.forEach((item, index) => {
                const x = margin.left + (graphWidth * item.hour / 23);
                const rectWidth = graphWidth / 23;
                const rectX = x - (rectWidth / 2);

                svg += '<rect x="' + rectX + '" y="' + margin.top + '" width="' + rectWidth + '" height="' + graphHeight + '" fill="transparent" ';
                svg += 'onmouseover="showTooltip(event, ' + item.hour + ', ' + item.price + ')" ';
                svg += 'onmousemove="moveTooltip(event)" ';
                svg += 'onmouseout="hideTooltip()"/>';
            });

            svg += '</svg>';
            document.getElementById('graph').innerHTML = svg;
        }

        function showTooltip(event, hour, price) {
            const tooltip = document.getElementById('tooltip');
            tooltip.style.display = 'block';
            tooltip.textContent = hour.toString().padStart(2, '0') + ':00 ' + price.toFixed(1) + ' øre/kWh';
            moveTooltip(event);
        }

        function moveTooltip(event) {
            const tooltip = document.getElementById('tooltip');
            tooltip.style.left = (event.pageX + 10) + 'px';
            tooltip.style.top = (event.pageY - 30) + 'px';
        }

        function hideTooltip() {
            document.getElementById('tooltip').style.display = 'none';
        }

        document.addEventListener('DOMContentLoaded', function() {
            loadData();
        });

        window.addEventListener('resize', function() {
            // Only redraw if we have cached data
            if (chartData) {
                createGraph(chartData);
            }
        });
    </script>
</body>
</html>
    "#,
    )
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/prices", get(prices));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
