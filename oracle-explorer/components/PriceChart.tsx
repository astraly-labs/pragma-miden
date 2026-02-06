"use client";

import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, Area, AreaChart } from 'recharts';

interface PriceDataPoint {
  timestamp: number;
  price: number;
}

interface PriceChartProps {
  data: PriceDataPoint[];
  pair: string;
}

export function PriceChart({ data, pair }: PriceChartProps) {
  const chartData = data.map(point => ({
    time: new Date(point.timestamp * 1000).toLocaleTimeString('en-US', { 
      hour: '2-digit', 
      minute: '2-digit',
      hour12: false 
    }),
    price: point.price,
    timestamp: point.timestamp,
  }));

  const formatPrice = (value: number) => {
    if (value < 1) {
      return `$${value.toFixed(6)}`;
    } else if (value < 100) {
      return `$${value.toFixed(4)}`;
    } else {
      return `$${value.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
    }
  };

  const minPrice = Math.min(...data.map(d => d.price));
  const maxPrice = Math.max(...data.map(d => d.price));
  const priceRange = maxPrice - minPrice;
  const yMin = minPrice - (priceRange * 0.05);
  const yMax = maxPrice + (priceRange * 0.05);

  if (data.length === 0) {
    return (
      <div className="w-full h-[400px] flex items-center justify-center text-text-muted">
        <p>No historical data available for {pair}</p>
      </div>
    );
  }

  return (
    <div className="w-full h-[450px]">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData} margin={{ top: 10, right: 30, left: 10, bottom: 0 }}>
          <defs>
            <linearGradient id="colorPrice" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#ffffff" stopOpacity={0.15}/>
              <stop offset="95%" stopColor="#ffffff" stopOpacity={0}/>
            </linearGradient>
          </defs>
          <XAxis 
            dataKey="time" 
            stroke="#3a3a3a"
            tick={{ fill: '#a0a0a0', fontSize: 11, fontWeight: 500 }}
            tickLine={{ stroke: '#2a2a2a' }}
            axisLine={{ stroke: '#2a2a2a' }}
          />
          <YAxis 
            domain={[yMin, yMax]}
            stroke="#3a3a3a"
            tick={{ fill: '#a0a0a0', fontSize: 11, fontWeight: 500 }}
            tickLine={{ stroke: '#2a2a2a' }}
            axisLine={{ stroke: '#2a2a2a' }}
            tickFormatter={formatPrice}
            width={90}
          />
          <Tooltip 
            contentStyle={{ 
              backgroundColor: '#0a0a0a',
              border: '2px solid #2a2a2a',
              borderRadius: '12px',
              padding: '12px 16px',
              boxShadow: '0 10px 40px rgba(0,0,0,0.5)'
            }}
            labelStyle={{ color: '#a0a0a0', fontSize: 12, fontWeight: 600, marginBottom: 4 }}
            itemStyle={{ color: '#ffffff', fontSize: 14, fontWeight: 700 }}
            formatter={(value: number | undefined) => value !== undefined ? [formatPrice(value), 'Price'] : ['-', 'Price']}
            cursor={{ stroke: '#6b6b6b', strokeWidth: 1, strokeDasharray: '5 5' }}
          />
          <Area 
            type="monotone" 
            dataKey="price" 
            stroke="#ffffff" 
            strokeWidth={2.5}
            fill="url(#colorPrice)"
            activeDot={{ r: 6, fill: '#ffffff', stroke: '#0a0a0a', strokeWidth: 3 }}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
