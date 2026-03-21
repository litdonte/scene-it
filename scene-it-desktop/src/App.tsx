import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

function App() {
  const [storyboard, setStoryboard] = useState<any>(null);

  useEffect(() => {
    invoke("get_storyboard").then(setStoryboard);
  }, []);

  return <pre>{JSON.stringify(storyboard, null, 2)}</pre>;
}

export default App;
