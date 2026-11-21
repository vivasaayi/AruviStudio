// Give the container browser a secure localhost origin while the fixture stays
// on the isolated Compose network. This forwards bytes without modifying HTTP.
import net from "node:net";

const target = process.env.ARUVI_CI_FIXTURE_HOST;
if (!target) throw new Error("ARUVI_CI_FIXTURE_HOST is required");

net.createServer((client) => {
  const upstream = net.connect(8787, target);
  client.on("error", () => upstream.destroy());
  upstream.on("error", () => client.destroy());
  client.pipe(upstream).pipe(client);
}).listen(18787, "127.0.0.1");
