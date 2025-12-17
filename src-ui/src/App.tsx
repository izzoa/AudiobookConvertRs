import { Layout } from "@/components/Layout";
import { FileInput } from "@/components/FileInput";
import { MetadataEditor } from "@/components/MetadataEditor";

function App() {
  return (
    <Layout>
      <div className="p-6 space-y-6 max-w-4xl mx-auto">

        {/* Step 1: Import */}
        <section className="space-y-3">
          <h2 className="text-xl font-semibold tracking-tight">Step 1: Import Files</h2>
          <FileInput />
        </section>

        {/* Step 2: Metadata */}
        <section className="space-y-3">
          <h2 className="text-xl font-semibold tracking-tight">Step 2: Edit Metadata</h2>
          <MetadataEditor />
        </section>

      </div>
    </Layout>
  );
}

export default App;
