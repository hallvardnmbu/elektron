import { Elysia } from 'elysia';
import { staticPlugin } from '@elysiajs/static';
import { html } from '@elysiajs/html';
import { dirname, join } from 'path';

let __dirname = dirname(new URL(import.meta.url).pathname);
__dirname = __dirname.startsWith('/') && __dirname.includes(':') 
  ? __dirname.replace(/^\/([A-Z]):/, '$1:\\').replace(/\//g, '\\')
  : __dirname;

// Fetch electricity prices
async function fetchPrices(year, month, day, region) {
    const url = `https://www.hvakosterstrommen.no/api/v1/prices/${year}/${month.toString().padStart(2, '0')}-${day.toString().padStart(2, '0')}_${region}.json`;
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
}

// Helper function to render the page template
function renderPage(data) {
  const { chart } = data;

  return `<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="Hold styr på strømprisene i Norge.">
    <title>elektron</title>
    <link rel="stylesheet" href="/style.css">
</head>
<body>
    <div id="header">
        <span id="headerTitle">Strømpriser (øre/kWh) i</span>
        <span id="regionSelector">
            <select class="region-dropdown" id="regionDropdown">
                <option value="NO1">NO1</option>
                <option value="NO2" selected>NO2</option>
                <option value="NO3">NO3</option>
                <option value="NO4">NO4</option>
                <option value="NO5">NO5</option>
            </select>
        </span>
    </div>
    
    <div id="dateNavigation" style="display: none;">
        <button class="nav-button" id="prevButton">◀ Forrige</button>
        <div id="currentDate"></div>
        <button class="nav-button" id="nextButton">Neste ▶</button>
    </div>
    
    <div id="thresholdControls" style="display: none;">
        <label class="threshold-checkbox threshold-zero">
            <input type="checkbox" id="threshold0" />
            <span>0 øre</span>
        </label>
        <label class="threshold-checkbox threshold-fifty">
            <input type="checkbox" id="threshold50" />
            <span>Norgespris</span>
        </label>
        <label class="threshold-checkbox threshold-ninety">
            <input type="checkbox" id="threshold75" />
            <span>75 øre</span>
        </label>
    </div>
    
    <div class="loading" id="loading">Laster laster laster...</div>

    <div id="graphContainer" style="display: none;">
        <canvas id="priceGraph"></canvas>
    </div>

    <div id="error" class="error" style="display: none;"></div>
    <div id="statistics" style="display: none;"></div>

    <script>
        let chartData = ${JSON.stringify(chart)};
        let currentDate = new Date();
        let currentRegion = 'NO2';
        let thresholdStates = {
            zero: true,
            fifty: true,
            seventyFive: true
        };

        // dataObject is expected to be the full chartData array here.
        function graphPrice(dataObject, targetDate = null) {
            const canvas = document.getElementById('priceGraph');
            const ctx = canvas.getContext('2d');

            const parent = canvas.parentElement;
            const dpr = window.devicePixelRatio || 1;
            canvas.width = parent.clientWidth * dpr;
            canvas.height = parent.clientHeight * dpr;
            ctx.setTransform(1, 0, 0, 1, 0, 0); // Reset transform before scaling
            ctx.scale(dpr, dpr);

            // Use the target date or fall back to today
            const dateToUse = targetDate || new Date();
            const offset = dateToUse.getTimezoneOffset();
            const adjustedDate = new Date(dateToUse.getTime() - offset * 60 * 1000);
            const dateString = adjustedDate.toISOString().split('T')[0];

            // Filter the data for the specified date and prepare price and hour arrays
            let dailyData = dataObject.filter(item => item.time.startsWith(dateString));
            if (dailyData.length === 0) {
                ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);
                ctx.font = '14px JetBrainsMono, "JetBrains Mono", monospace';
                ctx.fillStyle = '#1D1C1A';
                ctx.textAlign = 'center';
                ctx.fillText('Hmm. Ingen data.', canvas.width / dpr / 2, canvas.height / dpr / 2);
                
                // Remove any existing hover functionality for no data case
                canvas.onmousemove = null;
                canvas.onmouseleave = null;
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
            const paddedMin = minPrice - (priceRange * 0.1);

            ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);
            ctx.font = '12px JetBrainsMono, "JetBrains Mono", monospace';
            ctx.fillStyle = '#000000';
            ctx.textAlign = 'right';
            ctx.textBaseline = 'middle';

            // Draw Y-axis labels
            const yAxisTicks = 6;
            for (let i = 0; i <= yAxisTicks; i++) {
                const value = paddedMin + (paddedMax - paddedMin) * i / yAxisTicks;
                const y = margin.top + graphHeight - (graphHeight * i / yAxisTicks);
                
                // Y-axis labels
                ctx.fillText(value.toFixed(1), margin.left - 10, y);
            }

            // Draw X-axis labels with responsive spacing
            ctx.textAlign = 'center';
            ctx.textBaseline = 'top';
            const xTickDenominator = stepData.length - 1 > 0 ? stepData.length - 1 : 1;
            
            // Calculate if there's enough space for all hour labels
            ctx.font = '12px JetBrainsMono, "JetBrains Mono", monospace';
            const sampleText = '00';
            const labelWidth = ctx.measureText(sampleText).width;
            const minLabelSpacing = labelWidth + 10; // Add some padding
            const availableSpacing = graphWidth / (stepData.length - 1);
            
            // Determine label step based on actual space availability
            let labelStep = 1;
            if (availableSpacing < minLabelSpacing) {
                if (availableSpacing < minLabelSpacing / 2) {
                    labelStep = 4; // Show every 4th hour when very cramped
                } else {
                    labelStep = 2; // Show every 2nd hour when somewhat cramped
                }
            }
            
            for (let i = 0; i < stepData.length; i++) {
                const x = margin.left + (graphWidth / xTickDenominator) * i;
                
                // X-axis labels - only show at specified intervals
                if (i % labelStep === 0 || i === stepData.length - 1) {
                    ctx.fillText(hours[i].toString().padStart(2, '0'), x, margin.top + graphHeight + 10);
                }
            }

            // Draw step price line
            for (let i = 0; i < stepData.length - 1; i++) {
                const x1 = margin.left + (graphWidth / xTickDenominator) * i;
                const x2 = margin.left + (graphWidth / xTickDenominator) * (i + 1);
                const y1 = margin.top + graphHeight - ((stepData[i].price - paddedMin) / (paddedMax - paddedMin)) * graphHeight;
                const y2 = margin.top + graphHeight - ((stepData[i + 1].price - paddedMin) / (paddedMax - paddedMin)) * graphHeight;
                
                const lineColor = '#1D1C1A';
                
                ctx.beginPath();
                // Draw horizontal line for current step
                ctx.moveTo(x1, y1);
                ctx.lineTo(x2, y1);
                // Draw vertical line to next step level
                ctx.lineTo(x2, y2);
                ctx.strokeStyle = lineColor;
                ctx.lineWidth = 2;
                ctx.stroke();
            }

            // Draw threshold lines
            drawThresholdLines(ctx, margin, graphWidth, graphHeight, paddedMin, paddedMax);

            // --- Hover Functionality ---
            const hoverLayerId = 'priceGraph-hover';
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
                hoverCtx.font = '12px JetBrainsMono, "JetBrains Mono", monospace';
                hoverCtx.fillStyle = '#1D1C1A';
                hoverCtx.textAlign = 'left';

                // Tooltip
                const text = hour.toString().padStart(2, '0') + ':00 - ' + price.toFixed(1) + ' øre';
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
                hoverCtx.strokeStyle = '#1D1C1A';
                hoverCtx.lineWidth = 2;
                hoverCtx.strokeRect(textX - 5, textY - 15, textMetrics.width + 10, 20);
                
                // Tooltip text
                hoverCtx.fillStyle = '#1D1C1A';
                hoverCtx.fillText(text, textX, textY);

                // Vertical line
                hoverCtx.beginPath();
                hoverCtx.moveTo(xStep, margin.top);
                hoverCtx.lineTo(xStep, margin.top + graphHeight);
                hoverCtx.strokeStyle = '#1D1C1A';
                hoverCtx.lineWidth = 2;
                hoverCtx.stroke();
                
                // Point marker
                hoverCtx.beginPath();
                hoverCtx.arc(xStep, yStep, 4, 0, 2 * Math.PI);
                hoverCtx.fillStyle = '#1D1C1A';
                hoverCtx.fill();
            };

            canvas.onmouseleave = function() {
                hoverCtx.clearRect(0, 0, hoverLayer.width / dpr, hoverLayer.height / dpr);
            };
        }

        function drawThresholdLines(ctx, margin, graphWidth, graphHeight, paddedMin, paddedMax) {
            const thresholds = [
                { value: 0, name: '0 øre', color: '#CC0000', enabled: thresholdStates.zero },
                { value: 50, name: 'Norgespris', color: '#008E00', enabled: thresholdStates.fifty },
                { value: 75, name: '75 øre', color: '#CC0000', enabled: thresholdStates.seventyFive }
            ];

            thresholds.forEach(threshold => {
                if (threshold.enabled && threshold.value >= paddedMin && threshold.value <= paddedMax) {
                    const y = margin.top + graphHeight - ((threshold.value - paddedMin) / (paddedMax - paddedMin)) * graphHeight;
                    
                    ctx.beginPath();
                    ctx.moveTo(margin.left, y);
                    ctx.lineTo(margin.left + graphWidth, y);
                    ctx.strokeStyle = threshold.color;
                    ctx.lineWidth = 2;
                    ctx.stroke();
                    
                    // Add threshold label
                    ctx.font = '11px JetBrainsMono, "JetBrains Mono", monospace';
                    ctx.fillStyle = threshold.color;
                    ctx.textAlign = 'left';
                    ctx.textBaseline = 'middle';
                    ctx.fillText(threshold.name, margin.left + 5, y - 10);
                }
            });
        }

        function updateThresholdStates() {
            thresholdStates.zero = document.getElementById('threshold0').checked;
            thresholdStates.fifty = document.getElementById('threshold50').checked;
            thresholdStates.seventyFive = document.getElementById('threshold75').checked;
            
            if (chartData) {
                graphPrice(chartData, currentDate);
            }
        }

        function updateRegion() {
            currentRegion = document.getElementById('regionDropdown').value;
            loadData(currentDate);
        }

        async function loadData(date = null) {
            const error = document.getElementById('error');
            const statistics = document.getElementById('statistics');
            const loading = document.getElementById('loading');
            const graphContainer = document.getElementById('graphContainer');
            const dateNavigation = document.getElementById('dateNavigation');
            const thresholdControls = document.getElementById('thresholdControls');

            error.style.display = 'none';
            statistics.style.display = 'none';
            loading.style.display = 'block';
            graphContainer.style.display = 'none';
            thresholdControls.style.display = 'none';

            try {
                let url = '/prices';
                if (date) {
                    url = '/prices/' + date.getFullYear() + '/' + (date.getMonth() + 1) + '/' + date.getDate() + '/' + currentRegion;
                }

                const response = await fetch(url, {
                    headers: {
                        'Accept': 'application/json',
                        'Cache-Control': 'no-cache'
                    }
                });
                if (!response.ok) {
                    throw new Error("Er du i fremtiden? Neste dags priser blir tilgjengelige rundt klokken 13.");
                }

                const priceData = await response.json();

                if (priceData.length === 0) {
                    throw new Error('Hmm. Ingen data.');
                }

                loading.style.display = 'none';
                displayData(priceData, date);
                graphContainer.style.display = 'block';
                statistics.style.display = 'flex';
                dateNavigation.style.display = 'flex';
                thresholdControls.style.display = 'flex';

                chartData = priceData;
                setTimeout(() => graphPrice(chartData, date), 100); // Allow DOM to update

            } catch (err) {
                loading.style.display = 'none';
                error.textContent = err.message;
                error.style.display = 'block';
            }
        }

        function displayData(priceData, date = null) {
            const prices = priceData.map(item => item.price);
            const avgPrice = prices.reduce((a, b) => a + b, 0) / prices.length;
            const maxPrice = Math.max(...prices);
            const minPrice = Math.min(...prices);

            const displayDate = date || new Date();
            const dateStr = displayDate.getDate().toString().padStart(2, '0') + '-' +
                        (displayDate.getMonth() + 1).toString().padStart(2, '0') + '-' +
                        displayDate.getFullYear();
            const headerTitle = 'Strømpriser (øre/kWh) den ' + dateStr + ' i';
            document.getElementById('headerTitle').textContent = headerTitle;

            const statisticsElement = document.getElementById('statistics');
            statisticsElement.innerHTML = 
                '<span>Min.: ' + minPrice.toFixed(1) + '</span>' +
                '<span>Gjn.: ' + avgPrice.toFixed(1) + '</span>' +
                '<span>Maks: ' + maxPrice.toFixed(1) + '</span>';

            // Update date navigation
            document.getElementById('currentDate').textContent = dateStr;
            updateNavigationButtons();
        }

        function updateNavigationButtons() {
            const prevButton = document.getElementById('prevButton');
            const nextButton = document.getElementById('nextButton');
            
            const today = new Date();
            const tomorrow = new Date(today);
            tomorrow.setDate(tomorrow.getDate() + 1);
            
            const minDate = new Date('2020-01-01');
            
            // Enable/disable previous button
            const prevDate = new Date(currentDate);
            prevDate.setDate(prevDate.getDate() - 1);
            prevButton.disabled = prevDate < minDate;
            
            // Enable/disable next button (can go to tomorrow but not beyond)
            const nextDate = new Date(currentDate);
            nextDate.setDate(nextDate.getDate() + 1);
            nextButton.disabled = nextDate > tomorrow;
        }

        function navigateDate(direction) {
            const newDate = new Date(currentDate);
            newDate.setDate(newDate.getDate() + direction);
            
            const today = new Date();
            const tomorrow = new Date(today);
            tomorrow.setDate(tomorrow.getDate() + 1);
            const minDate = new Date('2020-01-01');
            
            if (newDate >= minDate && newDate <= tomorrow) {
                currentDate = newDate;
                loadData(currentDate);
            }
        }

        document.addEventListener('DOMContentLoaded', function() {
            // Add event listeners for navigation buttons
            document.getElementById('prevButton').addEventListener('click', () => navigateDate(-1));
            document.getElementById('nextButton').addEventListener('click', () => navigateDate(1));
            
            // Add event listeners for threshold checkboxes
            document.getElementById('threshold0').addEventListener('change', updateThresholdStates);
            document.getElementById('threshold50').addEventListener('change', updateThresholdStates);
            document.getElementById('threshold75').addEventListener('change', updateThresholdStates);
            
            // Add event listener for region dropdown
            document.getElementById('regionDropdown').addEventListener('change', updateRegion);
            
            // Set default checkbox states
            document.getElementById('threshold0').checked = true;
            document.getElementById('threshold50').checked = true;
            document.getElementById('threshold75').checked = true;
            
            loadData();
        });

        // Debounced resize handler for better performance
        let resizeTimeout;
        window.addEventListener('resize', function() {
            if (chartData) {
                clearTimeout(resizeTimeout);
                resizeTimeout = setTimeout(() => graphPrice(chartData, currentDate), 150);
            }
        });
    </script>
</body>
</html>
`
}

const elektronApp = new Elysia()
  .use(staticPlugin({
    assets: "src/other/elektron/public",
    prefix: "/"
  }))
  .use(html())
  .get('/', () => renderPage({}))
  .get('/fonts/:filename', async ({ params }) => {
    try {
      const { filename } = params;
      
      // Security check - prevent directory traversal
      if (filename.includes('..') || filename.includes('/') || filename.includes('\\')) {
        return new Response('Invalid filename', { status: 400 });
      }
      
      const fontPath = join(__dirname, 'font', filename);
      const fontFile = Bun.file(fontPath);
      
      // Check if file exists
      if (!await fontFile.exists()) {
        return new Response('Font not found', { status: 404 });
      }
      
      let mimeType = 'application/octet-stream';
      if (filename.endsWith('.woff2')) mimeType = 'font/woff2';
      else if (filename.endsWith('.woff')) mimeType = 'font/woff';
      else if (filename.endsWith('.ttf')) mimeType = 'font/ttf';
      
      return new Response(fontFile, {
        headers: {
          'content-type': mimeType,
          'cache-control': 'public, max-age=31536000' // Cache for 1 year
        }
      });
    } catch (error) {
      return new Response('Font not found', { status: 404 });
    }
  })
  .get('/prices', async () => {
    try {
      const now = new Date();
      const data = await fetchPrices(now.getFullYear(), now.getMonth() + 1, now.getDate(), 'NO2');
      
      const chart = data.map(item => {
        const hour = new Date(item.time_start).getHours();
        return {
          hour,
          price: item.NOK_per_kWh * 100.0,
          time: item.time_start,
          price_nok: item.NOK_per_kWh,
          price_eur: item.EUR_per_kWh,
        };
      });
      
      return Response.json(chart);
    } catch (error) {
      return Response.json({ message: "Finner ikke noe data. :-(" }, { status: 500 });
    }
  })
  .get('/prices/:year/:month/:day/:region', async ({ params }) => {
    try {
      const { year, month, day, region } = params;
      
      // Validation
      const yearNum = parseInt(year);
      const monthNum = parseInt(month);
      const dayNum = parseInt(day);
      
      // Check for NaN values (invalid numeric inputs)
      if (isNaN(yearNum) || isNaN(monthNum) || isNaN(dayNum)) {
        return Response.json({ message: 'Year, month, and day must be valid numbers' }, { status: 400 });
      }
      
      if (yearNum < 2020 || yearNum > 2030) {
        return Response.json({ message: 'Year must be between 2020 and 2030' }, { status: 400 });
      }
      if (monthNum < 1 || monthNum > 12) {
        return Response.json({ message: 'Month must be between 1 and 12' }, { status: 400 });
      }
      
      // Validate day for the given month and year
      const daysInMonth = new Date(yearNum, monthNum, 0).getDate();
      if (dayNum < 1 || dayNum > daysInMonth) {
        return Response.json({ message: `Day must be between 1 and ${daysInMonth} for month ${monthNum}` }, { status: 400 });
      }
      
      if (!['NO1', 'NO2', 'NO3', 'NO4', 'NO5'].includes(region)) {
        return Response.json({ message: 'Region must be NO1-NO5' }, { status: 400 });
      }
      
      const data = await fetchPrices(yearNum, monthNum, dayNum, region);
      
      const chart = data.map(item => {
        const hour = new Date(item.time_start).getHours();
        return {
          hour,
          price: item.NOK_per_kWh * 100.0,
          time: item.time_start,
          price_nok: item.NOK_per_kWh,
          price_eur: item.EUR_per_kWh,
        };
      });
      
      return Response.json(chart);
    } catch (error) {
      return Response.json({ message: "Noe gikk galt." }, { status: 500 });
    }
  });

export default elektronApp;

if (import.meta.main) {
    elektronApp.listen(3000);
    console.log(`http://${elektronApp.server?.hostname}:${elektronApp.server?.port}`);
}