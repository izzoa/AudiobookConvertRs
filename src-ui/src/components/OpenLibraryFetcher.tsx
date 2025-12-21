import { useState } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { useApp } from "@/context/AppContext";
import * as OpenLibraryApi from "@/api/openLibrary";
import type { OpenLibrarySearchResult, OpenLibrarySelection } from "@/types/models";

export function OpenLibraryFetcher() {
    const { activeAudiobook, setActiveAudiobook } = useApp();
    const [query, setQuery] = useState("");
    const [results, setResults] = useState<OpenLibrarySearchResult[]>([]);
    const [isSearching, setIsSearching] = useState(false);
    const [isApplying, setIsApplying] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Prefill query from current metadata
    const handlePrefill = () => {
        if (!activeAudiobook) return;
        const meta = activeAudiobook.output_metadata;
        const parts = [meta.title, meta.artist].filter(Boolean);
        setQuery(parts.join(" "));
    };

    const handleSearch = async () => {
        if (!query.trim()) return;
        setIsSearching(true);
        setError(null);
        setResults([]);

        try {
            const searchResults = await OpenLibraryApi.openLibrarySearch(query, 10);
            setResults(searchResults);
            if (searchResults.length === 0) {
                setError("No results found. Try a different query.");
            }
        } catch (e) {
            console.error("Search failed:", e);
            setError(`Search failed: ${e}`);
        } finally {
            setIsSearching(false);
        }
    };

    const handleApply = async (result: OpenLibrarySearchResult, withCover: boolean) => {
        if (!activeAudiobook) return;
        setIsApplying(true);
        setError(null);

        try {
            // Convert result to selection
            const selection: OpenLibrarySelection = {
                key: result.key,
                title: result.title,
                author: result.author_name?.[0],
                year: result.first_publish_year,
                cover_id: result.cover_i,
                publisher: result.publisher?.[0],
                genre: result.subject?.[0],
            };

            const updated = await OpenLibraryApi.openLibraryApply(
                activeAudiobook.id,
                selection,
                withCover
            );
            setActiveAudiobook(updated);
            setResults([]); // Clear results after successful apply
        } catch (e) {
            console.error("Apply failed:", e);
            setError(`Failed to apply: ${e}`);
        } finally {
            setIsApplying(false);
        }
    };

    if (!activeAudiobook) return null;

    return (
        <div className="space-y-4 p-4 border rounded-lg bg-card">
            <Label className="text-base font-semibold">Fetch from Open Library</Label>

            <div className="flex gap-2">
                <div className="flex-1">
                    <Input
                        placeholder="Search for book title or author..."
                        value={query}
                        onChange={(e) => setQuery(e.target.value)}
                        onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                    />
                </div>
                <Button onClick={handlePrefill} variant="outline" size="sm">
                    Prefill
                </Button>
                <Button onClick={handleSearch} disabled={isSearching || !query.trim()}>
                    {isSearching ? "Searching..." : "Search"}
                </Button>
            </div>

            {error && (
                <div className="text-sm text-destructive p-2 border border-destructive/30 rounded">
                    {error}
                </div>
            )}

            {results.length > 0 && (
                <div className="space-y-2 max-h-96 overflow-y-auto">
                    <p className="text-sm text-muted-foreground">
                        Found {results.length} results:
                    </p>
                    {results.map((result, idx) => (
                        <div key={idx} className="p-3 border rounded flex gap-3 items-start">
                            {result.cover_i && (
                                <img
                                    src={`https://covers.openlibrary.org/b/id/${result.cover_i}-S.jpg`}
                                    alt={result.title}
                                    className="w-12 h-16 object-cover rounded"
                                />
                            )}
                            <div className="flex-1 min-w-0">
                                <p className="font-medium truncate">{result.title}</p>
                                <p className="text-sm text-muted-foreground">
                                    {result.author_name?.join(", ") || "Unknown author"}
                                </p>
                                {result.first_publish_year && (
                                    <p className="text-xs text-muted-foreground">
                                        {result.first_publish_year}
                                    </p>
                                )}
                            </div>
                            <div className="flex flex-col gap-1">
                                <Button
                                    size="sm"
                                    onClick={() => handleApply(result, false)}
                                    disabled={isApplying}
                                >
                                    Metadata
                                </Button>
                                <Button
                                    size="sm"
                                    variant="secondary"
                                    onClick={() => handleApply(result, true)}
                                    disabled={isApplying || !result.cover_i}
                                >
                                    + Cover
                                </Button>
                            </div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}

