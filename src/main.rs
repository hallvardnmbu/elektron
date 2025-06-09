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

            display: flex;
            flex-direction: column;
            align-items: center;
        }

        #header {
            margin-bottom: 10px;
            font-weight: bold;
        }
        #statistics {
            white-space: pre;
            margin-top: 20px;
        }

        #graphContainer, #priceGraph {
            width: 100%;
        }
        #graphContainer {
            margin-top: 30px;
        }
        #priceGraph {
            height: 400px;
        }

        .error {
            border: 1px solid black;
            padding: 10px;
            margin: 10px 0;
        }

        }
    </style>
</head>
<body>
    <div id="header"></div>

    <div id="graphContainer">
        <canvas id="priceGraph"></canvas>
    </div>

    <div id="error" class="error" style="display: none;"></div>
    <div id="statistics"></div>

    <script>
        let chartData = null;

        // dataObject is expected to be the full chartData array here.
        // Always use today's date for filtering
        function graphPrice(dataObject) {
            const canvas = document.getElementById('priceGraph');
            const ctx = canvas.getContext('2d');

            const parent = canvas.parentElement;
            const dpr = window.devicePixelRatio || 1;
            canvas.width = parent.clientWidth * dpr;
            canvas.height = parent.clientHeight * dpr;
            ctx.setTransform(1, 0, 0, 1, 0, 0); // Reset transform before scaling
            ctx.scale(dpr, dpr);

            // Always use today's date (local time)
            const now = new Date();
            const offset = now.getTimezoneOffset();
            const adjustedDate = new Date(now.getTime() - offset * 60 * 1000);
            const todayString = adjustedDate.toISOString().split('T')[0];

            // Filter the data for today and prepare price and hour arrays
            let dailyData = dataObject.filter(item => item.time.startsWith(todayString));
            if (dailyData.length === 0) {
                ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);
                ctx.font = '16px JetBrains Mono';
                ctx.fillStyle = 'black';
                ctx.textAlign = 'center';
                ctx.fillText('No data available for this date.', canvas.width / dpr / 2, canvas.height / dpr / 2);
                return;
            }

            // Step graph: add a final point at last hour + 1 with the same value and correct hour label
            let stepData = dailyData.map(item => ({ hour: item.hour, price: item.price }));
            const last = stepData[stepData.length - 1];
            // Add a new hour label for the last value
            stepData.push({ hour: last.hour + 1, price: last.price });

            const prices = stepData.map(item => item.price);
            const hours = stepData.map(item => item.hour);

            const margin = { top: 20, right: 20, bottom: 30, left: 50 };
            const graphWidth = canvas.width / dpr - margin.left - margin.right;
            const graphHeight = canvas.height / dpr - margin.top - margin.bottom;

            const maxPrice = Math.max(...prices);
            const minPrice = Math.min(...prices);

            ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);
            ctx.font = '12px JetBrains Mono';
            ctx.fillStyle = 'black';

            // Draw Y-axis labels only (no grid or axis lines)
            const yAxisTicks = 5;
            for (let i = 0; i <= yAxisTicks; i++) {
                const value = minPrice + (maxPrice - minPrice) * i / yAxisTicks;
                const y = margin.top + graphHeight - (graphHeight * i / yAxisTicks);
                ctx.fillText(value.toFixed(2), margin.left - 40, y + 4);
            }

            // Draw X-axis labels only (no grid or axis lines)
            const xTickDenominator = stepData.length - 1 > 0 ? stepData.length - 1 : 1;
            for (let i = 0; i < stepData.length; i++) {
                const x = margin.left + (graphWidth / xTickDenominator) * i;
                ctx.fillText(hours[i].toString().padStart(2, '0'), x - 6, margin.top + graphHeight + 20);
            }

            // Draw step price line
            ctx.beginPath();
            for (let i = 0; i < stepData.length - 1; i++) {
                const x1 = margin.left + (graphWidth / xTickDenominator) * i;
                const x2 = margin.left + (graphWidth / xTickDenominator) * (i + 1);
                const y = margin.top + graphHeight - ((stepData[i].price - minPrice) / (maxPrice - minPrice)) * graphHeight;
                if (i === 0) {
                    ctx.moveTo(x1, y);
                } else {
                    ctx.lineTo(x1, y);
                }
                ctx.lineTo(x2, y); // horizontal step
            }
            ctx.strokeStyle = 'black';
            ctx.lineWidth = 2;
            ctx.stroke();

            // --- Hover Functionality ---
            const hoverLayerId = `priceGraph-hover`;
            let existingHoverLayer = document.getElementById(hoverLayerId);
            if (existingHoverLayer) {
                existingHoverLayer.remove();
            }

            let hoverLayer = document.createElement('canvas');
            hoverLayer.id = hoverLayerId;
            hoverLayer.style.position = "absolute";
            // Position hoverLayer exactly over the canvas
            hoverLayer.style.left = canvas.offsetLeft + "px";
            hoverLayer.style.top = canvas.offsetTop + "px";
            hoverLayer.width = canvas.width;
            hoverLayer.height = canvas.height;
            hoverLayer.style.width = canvas.width / dpr + "px";
            hoverLayer.style.height = canvas.height / dpr + "px";
            hoverLayer.style.pointerEvents = "none";
            canvas.parentElement.appendChild(hoverLayer);
            let hoverCtx = hoverLayer.getContext("2d");
            hoverCtx.setTransform(1, 0, 0, 1, 0, 0);
            hoverCtx.scale(dpr, dpr);

            canvas.onmousemove = function(event) {
                if (stepData.length < 2) return;
                const rect = canvas.getBoundingClientRect();
                const x = (event.clientX - rect.left) * (canvas.width / dpr / rect.width);
                // Find which step (hour) the mouse is over
                const xScale = graphWidth / xTickDenominator;
                let hoverIndex = Math.floor((x - margin.left) / xScale);
                if (hoverIndex < 0) hoverIndex = 0;
                if (hoverIndex >= stepData.length - 1) hoverIndex = stepData.length - 2;
                const price = stepData[hoverIndex].price;
                const hour = hours[hoverIndex];
                const xStep = margin.left + xScale * hoverIndex;
                const yStep = margin.top + graphHeight - ((price - minPrice) / (maxPrice - minPrice)) * graphHeight;

                hoverCtx.clearRect(0, 0, hoverLayer.width / dpr, hoverLayer.height / dpr);
                hoverCtx.font = `12px 'JetBrains Mono'`;
                hoverCtx.fillStyle = "black";
                hoverCtx.textAlign = "left";

                // Tooltip position: above the step
                let textX = xStep + 5;
                let textY = yStep - 10;
                const text = `${price.toFixed(1)}`;
                const textMetrics = hoverCtx.measureText(text);
                if (textX + textMetrics.width > canvas.width / dpr) {
                    textX = xStep - textMetrics.width - 10;
                }
                if (textY - 12 < 0) {
                    textY = yStep + 20;
                }
                hoverCtx.fillText(text, textX, textY);

                // Draw vertical line at the left edge of the step
                hoverCtx.beginPath();
                hoverCtx.moveTo(xStep, margin.top);
                hoverCtx.lineTo(xStep, margin.top + graphHeight);
                hoverCtx.strokeStyle = 'black';
                hoverCtx.lineWidth = 2;
                hoverCtx.stroke();
            };

            canvas.onmouseleave = function() {
                hoverCtx.clearRect(0, 0, hoverLayer.width / dpr, hoverLayer.height / dpr);
            };
        }

        async function loadData() {
            const error = document.getElementById('error');
            const statistics = document.getElementById('statistics');

            error.style.display = 'none';
            statistics.innerHTML = '';

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

                chartData = priceData; // chartData is the full data from /prices
                graphPrice(chartData); // Always use today's data

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
            const header = `Priser den ${dateStr} for NO2 i øre / kWh`;
            document.getElementById('header').textContent = header;

            const statistics = `Maks ${maxPrice.toFixed(1)} • Gjn. ${avgPrice.toFixed(1)} • Min. ${minPrice.toFixed(1)}`;
            document.getElementById('statistics').textContent = statistics;
        }

        document.addEventListener('DOMContentLoaded', function() {
            loadData();
        });

        window.addEventListener('resize', function() {
            // Only redraw if we have cached data
            if (chartData) {
                graphPrice(chartData); // Always use today's data
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
