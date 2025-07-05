import express from 'express';
import axios from 'axios';
import path from 'path';
import { fileURLToPath } from 'url';
import fs from 'fs/promises';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export async function elektronApp() {
    const app = express();

    // Serve static font files
    app.get('/fonts/:filename', async (req, res) => {
        try {
            const filename = req.params.filename;
            // Security check
            if (filename.includes('..') || filename.includes('/') || filename.includes('\\')) {
                return res.status(400).send('Invalid filename');
            }
            
            const fontPath = path.join(__dirname, 'src/font', filename);
            const fontData = await fs.readFile(fontPath);
            
            let mimeType = 'application/octet-stream';
            if (filename.endsWith('.woff2')) mimeType = 'font/woff2';
            else if (filename.endsWith('.woff')) mimeType = 'font/woff';
            else if (filename.endsWith('.ttf')) mimeType = 'font/ttf';
            
            res.setHeader('content-type', mimeType);
            res.setHeader('cache-control', 'public, max-age=31536000');
            res.send(fontData);
        } catch (error) {
            res.status(404).send('Font not found');
        }
    });

    // Fetch electricity prices
    async function fetchPrices(year, month, day, region) {
        const url = `https://www.hvakosterstrommen.no/api/v1/prices/${year}/${month.toString().padStart(2, '0')}-${day.toString().padStart(2, '0')}_${region}.json`;
        const response = await axios.get(url);
        return response.data;
    }

    app.get('/prices', async (req, res) => {
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
            
            res.json(chart);
        } catch (error) {
            res.status(500).json({ message: "Finner ikke noe data. :-(" });
        }
    });

    app.get('/prices/:year/:month/:day/:region', async (req, res) => {
        try {
            const { year, month, day, region } = req.params;
            
            // Validation
            if (year < 2020 || year > 2030) {
                return res.status(400).json({ message: 'Year must be between 2020 and 2030' });
            }
            if (month < 1 || month > 12) {
                return res.status(400).json({ message: 'Month must be between 1 and 12' });
            }
            if (!['NO1', 'NO2', 'NO3', 'NO4', 'NO5'].includes(region)) {
                return res.status(400).json({ message: 'Region must be NO1-NO5' });
            }
            
            const data = await fetchPrices(year, month, day, region);
            
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
            
            res.json(chart);
        } catch (error) {
            res.status(500).json({ message: "Noe gikk galt." });
        }
    });

    // Serve the HTML page
    app.get('/', (req, res) => {
        res.sendFile(path.join(__dirname, 'index.html'));
    });
    
    return app;
}