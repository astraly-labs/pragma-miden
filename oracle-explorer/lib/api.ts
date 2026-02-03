import type { Asset } from "@/types/asset";

export async function fetchPrices(): Promise<Asset[]> {
  try {
    const response = await fetch('/api/prices', {
      cache: 'no-store',
    });

    if (!response.ok) {
      throw new Error('Failed to fetch prices from API');
    }

    return await response.json();
  } catch (error) {
    console.error('Error fetching prices:', error);
    return [];
  }
}
