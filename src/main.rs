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

        .error {
            border: 1px solid black;
            padding: 10px;
            margin: 10px 0;
        }

        }
    </style>
</head>
<body>
    <div id="date"></div>

    <div id="error" class="error" style="display: none;"></div>
    <div id="data"></div>

    <div id="graphContainer" style="margin-top: 30px;">
        <div style="margin-bottom: 10px;">
            <button id="btnToday">Today</button>
            <button id="btnTomorrow">Tomorrow</button>
            <button id="btnDayAfterTomorrow">Day After Tomorrow</button>
        </div>
        <canvas id="priceGraphToday" style="width: 100%; height: 200px;"></canvas>
        <canvas id="priceGraphTomorrow" style="width: 100%; height: 200px; display: none;"></canvas>
        <canvas id="priceGraphDayAfterTomorrow" style="width: 100%; height: 200px; display: none;"></canvas>
    </div>

    <script>
        let chartData = null;
        const _RESOLUTION = 4;

        function formatDateAsLocalString(date) {
            const offset = date.getTimezoneOffset();
            const adjustedDate = new Date(date.getTime() - offset * 60 * 1000);
            return adjustedDate.toISOString().split('T')[0];
        }

        function generateDates() {
            const today = new Date();
            const tomorrow = new Date(today);
            tomorrow.setDate(today.getDate() + 1);
            const dayAfterTomorrow = new Date(tomorrow);
            dayAfterTomorrow.setDate(tomorrow.getDate() + 1);

            return {
                today: formatDateAsLocalString(today),
                tomorrow: formatDateAsLocalString(tomorrow),
                dayAfterTomorrow: formatDateAsLocalString(dayAfterTomorrow),
            };
        }

        // dataObject is expected to be the full chartData array here.
        // filterDateString is YYYY-MM-DD to filter items from dataObject.
        function graphPrice(canvasId, dataObject, filterDateString) {
            const canvas = document.getElementById(canvasId);
            if (!canvas) {
                console.error(`Canvas element with id '${canvasId}' not found.`);
                return;
            }
            const ctx = canvas.getContext('2d');

            const parent = canvas.parentElement;
            const dpr = window.devicePixelRatio || 1;
            canvas.width = parent.clientWidth * dpr;
            canvas.height = parent.clientHeight * dpr;
            ctx.scale(dpr, dpr);

            // Filter the data for the specific date and prepare price and hour arrays
            const dailyData = dataObject.filter(item => item.time.startsWith(filterDateString));
            const prices = dailyData.map(item => item.price); // Use item.price (øre)
            const axisHours = dailyData.map(item => item.hour.toString().padStart(2, '0'));

            if (prices.length === 0) {
                ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr); // Clear canvas if no data
                ctx.font = '16px JetBrains Mono';
                ctx.fillStyle = 'black';
                ctx.textAlign = 'center';
                ctx.fillText('No data available for this date.', canvas.width / dpr / 2, canvas.height / dpr / 2);
                return;
            }

            const margin = { top: 20, right: 20, bottom: 30, left: 50 };
            const graphWidth = canvas.width / dpr - margin.left - margin.right;
            const graphHeight = canvas.height / dpr - margin.top - margin.bottom;

            const maxPrice = Math.max(...prices);
            const minPrice = Math.min(...prices);

            ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);
            ctx.font = '12px JetBrains Mono';
            ctx.fillStyle = 'black';

            // Draw Y-axis labels and grid lines
            const yAxisTicks = 5;
            for (let i = 0; i <= yAxisTicks; i++) {
                const value = minPrice + (maxPrice - minPrice) * i / yAxisTicks;
                const y = margin.top + graphHeight - (graphHeight * i / yAxisTicks);
                ctx.fillText(value.toFixed(2), margin.left - 40, y + 4);

                ctx.beginPath();
                ctx.moveTo(margin.left, y);
                ctx.lineTo(margin.left + graphWidth, y);
                ctx.strokeStyle = '#eee';
                ctx.stroke();
            }

            // Draw X-axis labels and grid lines
            // Ensure there's at least one data point to prevent division by zero if prices.length is 1
            const xTickDenominator = prices.length > 1 ? prices.length - 1 : 1;
            for (let i = 0; i < prices.length; i++) {
                const x = margin.left + (graphWidth / xTickDenominator) * i;
                ctx.fillText(axisHours[i], x - 6, margin.top + graphHeight + 20);

                ctx.beginPath();
                ctx.moveTo(x, margin.top);
                ctx.lineTo(x, margin.top + graphHeight);
                ctx.strokeStyle = '#eee';
                ctx.stroke();
            }


            // Draw price line
            ctx.beginPath();
            prices.forEach((price, index) => {
                const x = margin.left + (graphWidth / xTickDenominator) * index;
                const y = margin.top + graphHeight - ((price - minPrice) / (maxPrice - minPrice)) * graphHeight;
                if (index === 0) {
                    ctx.moveTo(x, y);
                } else {
                    ctx.lineTo(x, y);
                }
            });
            ctx.strokeStyle = 'black';
            ctx.lineWidth = 2;
            ctx.stroke();

            // --- Hover Functionality ---
            const hoverLayerId = `${canvasId}-hover`;
            let existingHoverLayer = document.getElementById(hoverLayerId);
            if (existingHoverLayer) {
                existingHoverLayer.remove();
            }

            let hoverLayer = document.createElement('canvas');
            hoverLayer.id = hoverLayerId;
            hoverLayer.style.position = "absolute";
            // Position hoverLayer relative to the canvas's parent container, then align with canvas
            const canvasRect = canvas.getBoundingClientRect();
            const parentRect = canvas.parentElement.getBoundingClientRect();
            hoverLayer.style.left = (canvasRect.left - parentRect.left) + "px";
            hoverLayer.style.top = (canvasRect.top - parentRect.top) + "px";

            hoverLayer.width = canvas.width; // Use scaled width/height
            hoverLayer.height = canvas.height; // Use scaled width/height
            hoverLayer.style.width = canvas.style.width; // CSS width
            hoverLayer.style.height = canvas.style.height; // CSS height
            hoverLayer.style.pointerEvents = "none";
            canvas.parentElement.appendChild(hoverLayer);
            let hoverCtx = hoverLayer.getContext("2d");
            hoverCtx.scale(dpr, dpr); // Scale hover context same as main context

            canvas.addEventListener('mousemove', function(event) {
                if (prices.length === 0) return; // No data to hover over

                const rect = canvas.getBoundingClientRect();
                // Scale mouse physical pixels to canvas logical pixels
                const x = (event.clientX - rect.left) * (canvas.width / dpr / rect.width);
                const y = (event.clientY - rect.top) * (canvas.height / dpr / rect.height);

                const xScale = graphWidth / xTickDenominator;
                let hoverIndex = Math.round((x - margin.left) / xScale);

                if (hoverIndex >= 0 && hoverIndex < prices.length) {
                    const hoverPrice = prices[hoverIndex];
                    const hoverHour = axisHours[hoverIndex];
                    const pointX = margin.left + hoverIndex * xScale; // X-coordinate on the graph for this point

                    hoverCtx.clearRect(0, 0, hoverLayer.width / dpr, hoverLayer.height / dpr);
                    hoverCtx.font = `12px 'JetBrains Mono'`; // Font size in logical pixels
                    hoverCtx.fillStyle = "black";
                    hoverCtx.textAlign = "left";

                    // Adjust text position to be near cursor but constrained within canvas
                    let textX = (event.clientX - rect.left) + 15; // Logical pixels for tooltip text
                    let textY = (event.clientY - rect.top) - 15;

                    // Basic boundary check for tooltip text
                    const textMetrics = hoverCtx.measureText(`${hoverPrice.toFixed(1)} øre (${hoverHour})`);
                    if (textX + textMetrics.width > canvas.width / dpr) {
                        textX = (event.clientX - rect.left) - textMetrics.width - 15;
                    }
                    if (textY - 12 < 0) { // 12 is approx font height
                        textY = (event.clientY - rect.top) + 15 + 12;
                    }


                    hoverCtx.fillText(`${hoverPrice.toFixed(1)} øre (${hoverHour})`, textX, textY);

                    // Draw vertical line at the data point's x-coordinate
                    hoverCtx.beginPath();
                    hoverCtx.moveTo(pointX, margin.top); // Use margin.top
                    hoverCtx.lineTo(pointX, margin.top + graphHeight); // Use margin.top + graphHeight
                    hoverCtx.strokeStyle = '#888888';
                    hoverCtx.lineWidth = 1; // Logical pixel width
                    hoverCtx.stroke();
                } else {
                    hoverCtx.clearRect(0, 0, hoverLayer.width / dpr, hoverLayer.height / dpr);
                }
            });

            canvas.addEventListener('mouseleave', function() {
                hoverCtx.clearRect(0, 0, hoverLayer.width / dpr, hoverLayer.height / dpr);
            });
        }

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

                chartData = priceData; // chartData is the full data from /prices
                const dates = generateDates();
                graphPrice('priceGraphToday', chartData, dates.today);

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

        document.addEventListener('DOMContentLoaded', function() {
            loadData();
        });

        window.addEventListener('resize', function() {
            // Only redraw if we have cached data
            if (chartData) {
                const dates = generateDates();
                // Determine active graph and redraw it
                if (document.getElementById('priceGraphToday').style.display !== 'none') {
                    graphPrice('priceGraphToday', chartData, dates.today);
                } else if (document.getElementById('priceGraphTomorrow').style.display !== 'none') {
                    graphPrice('priceGraphTomorrow', chartData, dates.tomorrow);
                } else if (document.getElementById('priceGraphDayAfterTomorrow').style.display !== 'none') {
                    graphPrice('priceGraphDayAfterTomorrow', chartData, dates.dayAfterTomorrow);
                }
            }
        });

        document.getElementById('btnToday').addEventListener('click', () => {
            document.getElementById('priceGraphToday').style.display = 'block';
            document.getElementById('priceGraphTomorrow').style.display = 'none';
            document.getElementById('priceGraphDayAfterTomorrow').style.display = 'none';
            const dates = generateDates();
            graphPrice('priceGraphToday', chartData, dates.today);
        });

        document.getElementById('btnTomorrow').addEventListener('click', () => {
            document.getElementById('priceGraphToday').style.display = 'none';
            document.getElementById('priceGraphTomorrow').style.display = 'block';
            document.getElementById('priceGraphDayAfterTomorrow').style.display = 'none';
            const dates = generateDates();
            graphPrice('priceGraphTomorrow', chartData, dates.tomorrow);
        });

        document.getElementById('btnDayAfterTomorrow').addEventListener('click', () => {
            document.getElementById('priceGraphToday').style.display = 'none';
            document.getElementById('priceGraphTomorrow').style.display = 'none';
            document.getElementById('priceGraphDayAfterTomorrow').style.display = 'block';
            const dates = generateDates();
            graphPrice('priceGraphDayAfterTomorrow', chartData, dates.dayAfterTomorrow);
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
