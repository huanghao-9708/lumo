"""Export generated LUMO logo demos as preview PNGs and multi-size ICO files."""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "resources" / "branding" / "logo-demos"
SOURCES = {
    1: Path(r"C:\Users\hao\.codex\generated_images\01a0c6bf-c77e-77d0-9590-2780cd2c3e9c\exec-9a43fbf2-0614-4285-a2d2-7ef0621662c8.png"),
    2: Path(r"C:\Users\hao\.codex\generated_images\01a0c6bf-c77e-77d0-9590-2780cd2c3e9c\exec-4fb0aecc-44fa-44f2-a101-a4cf984afbd3.png"),
    3: Path(r"C:\Users\hao\.codex\generated_images\01a0c6bf-c77e-77d0-9590-2780cd2c3e9c\exec-a4ff0d56-a207-4cd7-bf14-d2797df5ab8c.png"),
    4: Path(r"C:\Users\hao\.codex\generated_images\01a0c6bf-c77e-77d0-9590-2780cd2c3e9c\exec-3af9be92-3054-4157-ba14-5dd3e1a0377a.png"),
}
OUTPUT_DIR_V2 = ROOT / "resources" / "branding" / "logo-demos-v2"
SOURCES_V2 = {
    1: Path(r"C:\Users\hao\.codex\generated_images\01a0c6bf-c77e-77d0-9590-2780cd2c3e9c\exec-21f890a7-3221-418d-85d4-2399261f6de7.png"),
    2: Path(r"C:\Users\hao\.codex\generated_images\01a0c6bf-c77e-77d0-9590-2780cd2c3e9c\exec-f7636bc3-d3f7-4588-b2a0-7685108d01a2.png"),
    3: Path(r"C:\Users\hao\.codex\generated_images\01a0c6bf-c77e-77d0-9590-2780cd2c3e9c\exec-e258bf2f-bb7a-40b0-891d-fde215f96613.png"),
}
ICO_SIZES = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]


def export_demo(number: int, source: Path, output_dir: Path) -> None:
    image = Image.open(source).convert("RGBA")
    preview_path = output_dir / f"lumo-logo-demo-{number}.png"
    icon_path = output_dir / f"lumo-logo-demo-{number}.ico"
    image.save(preview_path, "PNG", optimize=True)
    image.save(icon_path, "ICO", sizes=ICO_SIZES, bitmap_format="png")


def export_contact_sheet(output_dir: Path, count: int) -> None:
    rows = (count + 1) // 2
    canvas = Image.new("RGB", (1200, 40 + rows * 580), "#F7F5F1")
    draw = ImageDraw.Draw(canvas)
    font_path = Path(r"C:\Windows\Fonts\segoeuib.ttf")
    font = ImageFont.truetype(font_path, 38) if font_path.exists() else ImageFont.load_default()

    for index in range(1, count + 1):
        row, column = divmod(index - 1, 2)
        card_x, card_y = 40 + column * 580, 40 + row * 580
        draw.rounded_rectangle(
            (card_x, card_y, card_x + 540, card_y + 540),
            radius=36,
            fill="#FBFAF7",
            outline="#E8E5DE",
            width=3,
        )
        icon = Image.open(output_dir / f"lumo-logo-demo-{index}.png").convert("RGBA")
        icon.thumbnail((430, 430), Image.Resampling.LANCZOS)
        icon_x = card_x + (540 - icon.width) // 2
        icon_y = card_y + 50 + (410 - icon.height) // 2
        canvas.paste(icon, (icon_x, icon_y), icon)
        draw.text((card_x + 28, card_y + 470), f"DEMO {index}", fill="#111111", font=font)
    canvas.save(output_dir / "lumo-logo-demos-preview.png", "PNG", optimize=True)


def export_set(sources: dict[int, Path], output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    for number, source in sources.items():
        export_demo(number, source, output_dir)
    export_contact_sheet(output_dir, len(sources))


def main() -> None:
    export_set(SOURCES, OUTPUT_DIR)
    export_set(SOURCES_V2, OUTPUT_DIR_V2)


if __name__ == "__main__":
    main()
