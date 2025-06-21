use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Datelike, Local, Timelike};
use serde::{Deserialize, Serialize};
use tokio;
use std::path::Path;

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

async fn serve_font(axum::extract::Path(filename): axum::extract::Path<String>) -> Result<Response, StatusCode> {
    // Prevent path traversal attacks by sanitizing the filename
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Only allow specific font file extensions
    if !filename.ends_with(".woff2") && !filename.ends_with(".woff") && !filename.ends_with(".ttf") {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let font_dir = Path::new("src/font");
    let font_path = font_dir.join(&filename);
    
    // Ensure the resolved path is within the font directory
    let canonical_font_dir = font_dir.canonicalize().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let canonical_font_path = font_path.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    
    if !canonical_font_path.starts_with(&canonical_font_dir) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    if font_path.exists() {
        let font_data = tokio::fs::read(&font_path).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mime_type = if filename.ends_with(".woff2") {
            "font/woff2"
        } else if filename.ends_with(".woff") {
            "font/woff"
        } else if filename.ends_with(".ttf") {
            "font/ttf"
        } else {
            "application/octet-stream"
        };
        
        Ok(Response::builder()
            .header("content-type", mime_type)
            .header("cache-control", "public, max-age=31536000")
            .body(axum::body::Body::from(font_data))
            .unwrap())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
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
            src: url('/fonts/Regular.woff2') format('woff2');
            font-weight: 400;
            font-style: normal;
            font-display: swap;
        }
        @font-face {
            font-family: 'JetBrainsMono';
            src: url('/fonts/Bold.woff2') format('woff2');
            font-weight: 700;
            font-style: normal;
            font-display: swap;
        }
        @font-face {
            font-family: 'JetBrainsMono';
            src: url('/fonts/Light.woff2') format('woff2');
            font-weight: 300;
            font-style: normal;
            font-display: swap;
        }

        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            border-radius: 0 !important;
        }

        html, body {
            height: 100%;
            font-family: 'JetBrainsMono', 'JetBrains Mono', 'Courier New', monospace;
            background: #ffffff;
            color: #000000;
            line-height: 1.2;
            font-size: 14px;
            font-weight: 400;
        }

        body {
            display: flex;
            flex-direction: column;
            align-items: center;
            padding: 20px;
            min-height: 100vh;
        }

        #header {
            font-weight: 700;
            font-size: 16px;
            margin-bottom: 20px;
            text-align: center;
            letter-spacing: 0.5px;
            border-bottom: 2px solid #000000;
            padding-bottom: 10px;
            width: 100%;
            max-width: 800px;
        }

        #graphContainer {
            width: 100%;
            max-width: 800px;
            height: 400px;
            margin: 20px 0;
            border: 2px solid #000000;
            background: #ffffff;
            position: relative;
        }

        #priceGraph {
            width: 100%;
            height: 100%;
            display: block;
        }

        #statistics {
            font-weight: 700;
            font-size: 16px;
            margin-top: 20px;
            text-align: center;
            letter-spacing: 1px;
            border: 2px solid #000000;
            padding: 15px 20px;
            background: #ffffff;
            white-space: pre;
            min-width: 300px;
        }

        .error {
            border: 2px solid #000000;
            background: #ffffff;
            color: #000000;
            padding: 15px 20px;
            margin: 20px 0;
            font-weight: 700;
            text-align: center;
            max-width: 600px;
            width: 100%;
        }

        .error::before {
            content: "ERROR: ";
            font-weight: 700;
        }

        /* Ensure no corner radii anywhere */
        *, *::before, *::after {
            border-radius: 0 !important;
        }

        /* Loading state */
        .loading {
            font-weight: 300;
            text-align: center;
            padding: 20px;
        }

        /* Responsive adjustments */
        @media (max-width: 600px) {
            body {
                padding: 10px;
            }
            
            #header {
                font-size: 14px;
                margin-bottom: 15px;
            }
            
            #graphContainer {
                height: 300px;
                margin: 15px 0;
            }
            
            #statistics {
                font-size: 14px;
                padding: 10px 15px;
                margin-top: 15px;
            }
        }
    </style>
</head>
<body>
    <div id="header">ELEKTRON</div>
    <div class="loading" id="loading">LOADING DATA...</div>

    <div id="graphContainer" style="display: none;">
        <canvas id="priceGraph"></canvas>
    </div>

    <div id="error" class="error" style="display: none;"></div>
    <div id="statistics" style="display: none;"></div>

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
                ctx.font = '14px JetBrainsMono';
                ctx.fillStyle = '#000000';
                ctx.textAlign = 'center';
                ctx.fillText('NO DATA AVAILABLE FOR THIS DATE', canvas.width / dpr / 2, canvas.height / dpr / 2);
                return;
            }

            // Step graph: add a final point at last hour + 1 with the same value and correct hour label
            let stepData = dailyData.map(item => ({ hour: item.hour, price: item.price }));
            const last = stepData[stepData.length - 1];
            // Add a new hour label for the last value
            stepData.push({ hour: last.hour + 1, price: last.price });

            const prices = stepData.map(item => item.price);
            const hours = stepData.map(item => item.hour);

            const margin = { top: 30, right: 30, bottom: 40, left: 60 };
            const graphWidth = canvas.width / dpr - margin.left - margin.right;
            const graphHeight = canvas.height / dpr - margin.top - margin.bottom;

            const maxPrice = Math.max(...prices);
            const minPrice = Math.min(...prices);
            const priceRange = maxPrice - minPrice;
            const paddedMax = maxPrice + (priceRange * 0.1);
            const paddedMin = Math.max(0, minPrice - (priceRange * 0.1));

            ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);
            ctx.font = '12px JetBrainsMono';
            ctx.fillStyle = '#000000';
            ctx.textAlign = 'right';
            ctx.textBaseline = 'middle';

            // Draw Y-axis labels and grid lines
            const yAxisTicks = 6;
            for (let i = 0; i <= yAxisTicks; i++) {
                const value = paddedMin + (paddedMax - paddedMin) * i / yAxisTicks;
                const y = margin.top + graphHeight - (graphHeight * i / yAxisTicks);
                
                // Y-axis labels
                ctx.fillText(value.toFixed(1), margin.left - 10, y);
                
                // Horizontal grid lines
                ctx.beginPath();
                ctx.moveTo(margin.left, y);
                ctx.lineTo(margin.left + graphWidth, y);
                ctx.strokeStyle = i === 0 ? '#000000' : '#cccccc';
                ctx.lineWidth = i === 0 ? 2 : 1;
                ctx.stroke();
            }

            // Draw X-axis labels and grid lines
            ctx.textAlign = 'center';
            ctx.textBaseline = 'top';
            const xTickDenominator = stepData.length - 1 > 0 ? stepData.length - 1 : 1;
            for (let i = 0; i < stepData.length; i++) {
                const x = margin.left + (graphWidth / xTickDenominator) * i;
                
                // X-axis labels
                ctx.fillText(hours[i].toString().padStart(2, '0'), x, margin.top + graphHeight + 10);
                
                // Vertical grid lines
                if (i % 2 === 0) { // Show grid every 2 hours
                    ctx.beginPath();
                    ctx.moveTo(x, margin.top);
                    ctx.lineTo(x, margin.top + graphHeight);
                    ctx.strokeStyle = '#cccccc';
                    ctx.lineWidth = 1;
                    ctx.stroke();
                }
            }

            // Draw step price line
            ctx.beginPath();
            for (let i = 0; i < stepData.length - 1; i++) {
                const x1 = margin.left + (graphWidth / xTickDenominator) * i;
                const x2 = margin.left + (graphWidth / xTickDenominator) * (i + 1);
                const y = margin.top + graphHeight - ((stepData[i].price - paddedMin) / (paddedMax - paddedMin)) * graphHeight;
                if (i === 0) {
                    ctx.moveTo(x1, y);
                } else {
                    ctx.lineTo(x1, y);
                }
                ctx.lineTo(x2, y); // horizontal step
            }
            ctx.strokeStyle = '#000000';
            ctx.lineWidth = 3;
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
                const xScale = graphWidth / xTickDenominator;
                let hoverIndex = Math.floor((x - margin.left) / xScale);
                if (hoverIndex < 0) hoverIndex = 0;
                if (hoverIndex >= stepData.length - 1) hoverIndex = stepData.length - 2;
                const price = stepData[hoverIndex].price;
                const hour = hours[hoverIndex];
                const xStep = margin.left + xScale * hoverIndex;
                const yStep = margin.top + graphHeight - ((price - paddedMin) / (paddedMax - paddedMin)) * graphHeight;

                hoverCtx.clearRect(0, 0, hoverLayer.width / dpr, hoverLayer.height / dpr);
                hoverCtx.font = '12px JetBrainsMono';
                hoverCtx.fillStyle = '#000000';
                hoverCtx.textAlign = 'left';

                // Tooltip
                const text = `${hour.toString().padStart(2, '0')}:00 - ${price.toFixed(1)} øre`;
                const textMetrics = hoverCtx.measureText(text);
                let textX = xStep + 10;
                let textY = yStep - 15;
                
                if (textX + textMetrics.width > canvas.width / dpr - 10) {
                    textX = xStep - textMetrics.width - 10;
                }
                if (textY < 20) {
                    textY = yStep + 25;
                }
                
                // Tooltip background
                hoverCtx.fillStyle = '#ffffff';
                hoverCtx.fillRect(textX - 5, textY - 15, textMetrics.width + 10, 20);
                hoverCtx.strokeStyle = '#000000';
                hoverCtx.lineWidth = 2;
                hoverCtx.strokeRect(textX - 5, textY - 15, textMetrics.width + 10, 20);
                
                // Tooltip text
                hoverCtx.fillStyle = '#000000';
                hoverCtx.fillText(text, textX, textY);

                // Vertical line
                hoverCtx.beginPath();
                hoverCtx.moveTo(xStep, margin.top);
                hoverCtx.lineTo(xStep, margin.top + graphHeight);
                hoverCtx.strokeStyle = '#000000';
                hoverCtx.lineWidth = 2;
                hoverCtx.stroke();
                
                // Point marker
                hoverCtx.beginPath();
                hoverCtx.arc(xStep, yStep, 4, 0, 2 * Math.PI);
                hoverCtx.fillStyle = '#000000';
                hoverCtx.fill();
            };

            canvas.onmouseleave = function() {
                hoverCtx.clearRect(0, 0, hoverLayer.width / dpr, hoverLayer.height / dpr);
            };
        }

        async function loadData() {
            const error = document.getElementById('error');
            const statistics = document.getElementById('statistics');
            const loading = document.getElementById('loading');
            const graphContainer = document.getElementById('graphContainer');

            error.style.display = 'none';
            statistics.style.display = 'none';
            loading.style.display = 'block';
            graphContainer.style.display = 'none';

            try {
                const response = await fetch('/prices');
                if (!response.ok) {
                    throw new Error('HTTP ERROR ' + response.status);
                }

                const priceData = await response.json();

                if (priceData.length === 0) {
                    throw new Error('NO DATA AVAILABLE');
                }

                loading.style.display = 'none';
                displayData(priceData);
                graphContainer.style.display = 'block';
                statistics.style.display = 'block';

                chartData = priceData;
                setTimeout(() => graphPrice(chartData), 100); // Allow DOM to update

            } catch (err) {
                loading.style.display = 'none';
                error.textContent = err.message;
                error.style.display = 'block';
            }
        }

        function displayData(priceData) {
            const prices = priceData.map(item => item.price);
            const avgPrice = prices.reduce((a, b) => a + b, 0) / prices.length;
            const maxPrice = Math.max(...prices);
            const minPrice = Math.min(...prices);

            const now = new Date();
            const dateStr = now.getDate().toString().padStart(2, '0') + '-' +
                           (now.getMonth() + 1).toString().padStart(2, '0') + '-' +
                           now.getFullYear();
            const header = `ELECTRICITY PRICES ${dateStr} (NO2) - øre/kWh`;
            document.getElementById('header').textContent = header;

            const statistics = `MAX: ${maxPrice.toFixed(1)} • AVG: ${avgPrice.toFixed(1)} • MIN: ${minPrice.toFixed(1)}`;
            document.getElementById('statistics').textContent = statistics;
        }

        document.addEventListener('DOMContentLoaded', function() {
            loadData();
        });

        window.addEventListener('resize', function() {
            if (chartData) {
                setTimeout(() => graphPrice(chartData), 100);
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
        .route("/prices", get(prices))
        .route("/fonts/:filename", get(serve_font));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
