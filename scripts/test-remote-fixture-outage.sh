#!/usr/bin/env bash
# Sourced after stopping the disposable PDF renderer.
docker exec "$name" node -e '
  (async () => {
    const headers = {"Content-Type":"application/json", "Authorization":"Bearer workflow-mobile-test"};
    const product = await fetch("http://127.0.0.1:8787/api/mobile/products", {method:"POST", headers, body:JSON.stringify({name:"PDF renderer failure fixture"})});
    if (product.status !== 201) throw new Error("Could not create isolated failure fixture");
    const {id} = await product.json();
    const result = await fetch(`http://127.0.0.1:8787/api/mobile/products/${encodeURIComponent(id)}/book-export`, {method:"POST", headers, body:JSON.stringify({format:"pdf"})});
    if (result.status !== 400 || !(await result.text()).includes("unavailable")) throw new Error("PDF export did not fail closed when isolated renderer stopped");
    console.log("PDF renderer outage fails closed");
  })().catch(error => { console.error(error.message); process.exitCode=1; });'
