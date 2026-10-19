import { $, expect } from "@wdio/globals";

describe("Janela principal", () => {
  it("abre com o painel de prompt ou o setup", async () => {
    await expect(browser).toHaveTitle("Voxel");
    const prompt = await $("#prompt").isDisplayed();
    const setup = await $("#setup-title").isDisplayed();
    expect(prompt || setup).toBe(true);
    await expect($("footer")).toBeDisplayed();
  });
});
