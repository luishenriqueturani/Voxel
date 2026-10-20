import { useTranslation } from "react-i18next";

const TILES = ["1", "2", "3", "4", "5", "6", "7", "8", "9"] as const;

export function MosaicPreview({ src }: { src: string }) {
  const { t } = useTranslation();
  return (
    <fieldset
      data-testid="texture-mosaic"
      className="m-0 grid w-full max-w-3xl grid-cols-3 border-0 p-0"
    >
      <legend className="sr-only">{t("viewer.mosaicPreview")}</legend>
      {TILES.map((tile) => (
        <img key={tile} alt="" src={src} className="block w-full" />
      ))}
    </fieldset>
  );
}
