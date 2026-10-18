import { $, expect } from "@wdio/globals";

describe("Janela principal", () => {
  it("abre com o painel de prompt", async () => {
    await expect(browser).toHaveTitle("Voxel");
    await expect($("#prompt")).toBeDisplayed();
    await expect($("footer")).toBeDisplayed();
  });
});
