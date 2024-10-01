import { mockedSearchResult } from "./api-mock";
import { TraceResult } from "./shared/type";

export type SearchResult = {
  project_root: string;
  trace_result: TraceResult;
};

export async function getSearchResult(search: string): Promise<SearchResult> {
  const url = `http://127.0.0.1:8080/search/${encodeURIComponent(search)}`;

  if (search === "demo") {
    return mockedSearchResult;
  }

  try {
    const response = await fetch(url);

    if (!response.ok) {
      throw new Error(`Error: ${response.statusText}`);
    }

    const data = await response.json();

    return data;
  } catch (error) {
    // Handle and re-throw the error so React Query can capture it
    throw new Error(
      `Failed to fetch search results: ${(error as Error).message}`
    );
  }
}
