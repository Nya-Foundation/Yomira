import type { NetworkConfig } from "../types";

import { invokeOrThrow, isTauri, loadPreferences } from "./transport";

export async function getNetworkConfig(): Promise<NetworkConfig> {
  if (isTauri()) return invokeOrThrow("get_network_config");
  return (await loadPreferences()).getNetworkConfig();
}

export async function setNetworkConfig(config: NetworkConfig): Promise<boolean> {
  if (isTauri()) {
    await invokeOrThrow("set_network_config", { config });
    return true;
  }
  return (await loadPreferences()).setNetworkConfig(config);
}

export async function getBuiltinHosts(): Promise<string> {
  if (isTauri()) return invokeOrThrow("get_builtin_hosts");
  return (await import("../../../src-tauri/resources/hosts.txt?raw")).default;
}
