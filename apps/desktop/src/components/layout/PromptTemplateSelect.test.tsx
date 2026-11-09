import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PromptTemplateSelect } from "./PromptTemplateSelect";

describe("PromptTemplateSelect", () => {
  it("mostra a prévia e troca o template", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(
      <PromptTemplateSelect
        templates={[
          {
            id: "tpl",
            name: "Personagem",
            body: "T-pose",
            negative: "",
            placement: "after",
          },
        ]}
        templateId="tpl"
        prompt="ferreiro"
        negativePrompt=""
        onChange={onChange}
      />,
    );
    expect(screen.getByText("ferreiro, T-pose")).toBeInTheDocument();
    await user.selectOptions(screen.getByRole("combobox", { name: "Template" }), "");
    expect(onChange).toHaveBeenCalledWith("");
  });
});
