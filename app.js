import { Elysia } from 'elysia';

// Fetch electricity prices
async function fetchPrices(year, month, day, region) {
    const url = `https://www.hvakosterstrommen.no/api/v1/prices/${year}/${month.toString().padStart(2, '0')}-${day.toString().padStart(2, '0')}_${region}.json`;
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
}

const elektronApp = new Elysia()
    .get('/', () => {
        return new Response(Bun.file('index.html'), {
            headers: {
                'content-type': 'text/html'
            }
        });
    })
    .get('/fonts/:filename', async ({ params }) => {
        try {
            const { filename } = params;
            
            // Security check - prevent directory traversal
            if (filename.includes('..') || filename.includes('/') || filename.includes('\\')) {
                return new Response('Invalid filename', { status: 400 });
            }
            
            const fontPath = `font/${filename}`;
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
            
            if (yearNum < 2020 || yearNum > 2030) {
                return Response.json({ message: 'Year must be between 2020 and 2030' }, { status: 400 });
            }
            if (monthNum < 1 || monthNum > 12) {
                return Response.json({ message: 'Month must be between 1 and 12' }, { status: 400 });
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