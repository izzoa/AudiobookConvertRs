import { useState } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";

export function MetadataEditor() {
    const [title, setTitle] = useState("");
    const [author, setAuthor] = useState("");
    const [narrator, setNarrator] = useState("");

    return (
        <div className="space-y-4 p-4 border rounded-lg bg-card">
            <div className="grid gap-2">
                <Label htmlFor="title">Title</Label>
                <Input
                    id="title"
                    placeholder="Enter audiobook title"
                    value={title}
                    onChange={(e) => setTitle(e.target.value)}
                />
            </div>

            <div className="grid grid-cols-2 gap-4">
                <div className="grid gap-2">
                    <Label htmlFor="author">Author</Label>
                    <Input
                        id="author"
                        placeholder="Author name"
                        value={author}
                        onChange={(e) => setAuthor(e.target.value)}
                    />
                </div>
                <div className="grid gap-2">
                    <Label htmlFor="narrator">Narrator</Label>
                    <Input
                        id="narrator"
                        placeholder="Narrator name"
                        value={narrator}
                        onChange={(e) => setNarrator(e.target.value)}
                    />
                </div>
            </div>

            <div className="pt-2">
                <Button className="w-full sm:w-auto">Save Metadata</Button>
            </div>
        </div>
    );
}
