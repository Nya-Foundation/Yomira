export function formatHosts(hosts: Record<string, string[]>): string {
  return Object.entries(hosts).sort(([a], [b]) => a.localeCompare(b))
    .flatMap(([host, ips]) => ips.map((ip) => `${ip} ${host}`)).join("\n");
}

/** Standard hosts syntax: IPv4/IPv6, multiple aliases, comments and blank lines. */
export function parseHosts(text: string): Record<string, string[]> {
  const hosts: Record<string, string[]> = Object.create(null);
  text.split(/\r?\n/).forEach((line, index) => {
    const parts = line.split("#", 1)[0].trim().split(/\s+/);
    if (!parts[0]) return;
    const [ip, ...names] = parts;
    const validIp = ip.includes(":") ? validIpv6(ip) : /^\d{1,3}(\.\d{1,3}){3}$/.test(ip)
      && ip.split(".").every((part) => Number(part) <= 255 && String(Number(part)) === part);
    if (!validIp || names.length === 0 || names.some((name) => !validHost(name))) {
      throw new Error(`Line ${index + 1}: expected an IP address followed by hostnames.`);
    }
    for (const name of names) {
      const key = name.toLowerCase();
      const ips = hosts[key] ??= [];
      if (!ips.includes(ip)) ips.push(ip);
    }
  });
  return hosts;
}

function validIpv6(ip: string): boolean {
  try { return new URL(`http://[${ip}]/`).hostname.startsWith("["); } catch { return false; }
}
function validHost(host: string): boolean {
  return host.length <= 253 && host.split(".").every((label) =>
    /^[a-z\d](?:[a-z\d-]{0,61}[a-z\d])?$/i.test(label));
}
