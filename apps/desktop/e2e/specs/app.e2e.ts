import { $, expect } from "@wdio/globals";

describe("Janela principal", () => {
  it("abre exibindo o nome da aplicação", async () => {
    await expect($("h1")).toHaveText("Voxel");
  });
});
