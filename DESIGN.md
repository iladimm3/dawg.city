# Design System Specification: Editorial Playfulness

## 1. Overview & Creative North Star

### Creative North Star: "The Joyful Guardian"
This design system moves beyond the standard "pet app" aesthetic to create a high-end, editorial experience that feels both professional and deeply affectionate. We reject the "generic SaaS" look in favor of **Organic Playfulness**. 

The system is defined by massive, welcoming radii, a deep immersion in violet hues, and a rejection of traditional structural lines. We achieve sophistication through "intentional asymmetry"—placing secondary elements like paw and bone icons in a floating, non-grid-aligned manner—to mimic the natural, unpredictable movement of a happy pet. This isn't just a UI; it’s a tactile, immersive environment where depth is felt through color shifts rather than seen through borders.

---

## 2. Colors & Surface Architecture

The palette is anchored in a regal `surface` (#180429) that provides a canvas for high-energy accents.

### The "No-Line" Rule
**Explicit Instruction:** Do not use 1px solid borders to define sections. Layouts must be defined strictly through background color shifts. For example, a card should use `surface_container_high` sitting atop a `surface` background. This creates a soft, modern transition that mimics the way light falls on soft surfaces.

### Surface Hierarchy & Nesting
Treat the UI as layered sheets of polished acrylic. Use the following hierarchy for nesting:
*   **Base:** `surface` (#180429)
*   **Sectioning:** `surface_container_low` (#1e0831)
*   **Primary Interaction Cards:** `surface_container_high` (#2d1343)
*   **Floating Elements:** `surface_bright` (#3c1d56)

### The Glass & Gradient Rule
To ensure a premium feel, main CTAs should utilize a subtle linear gradient transitioning from `primary` (#be9dff) to `primary_dim` (#8b4ef7) at a 135-degree angle. For floating overlays or navigation bars, apply **Glassmorphism**: use `surface_variant` (#34184c) at 60% opacity with a `20px` backdrop blur.

---

## 3. Typography

The typography strategy balances the authority of a premium brand with the warmth of a friendly companion.

*   **Display & Headline (Plus Jakarta Sans):** Selected for its modern, geometric clarity with a hint of warmth. Bold weights should be used for `display-lg` through `headline-sm` to create an impactful editorial hierarchy.
*   **Body & Labels (Be Vietnam Pro):** Chosen for its exceptional legibility and slightly rounded terminals, which complement the "Playful" theme without sacrificing professional clarity.

**The Hierarchy Goal:** Use dramatic scale shifts. A `display-lg` headline should feel massive and welcoming, immediately contrasted by a clean, spacious `body-lg` paragraph. This high-contrast scale is the hallmark of high-end editorial design.

---

## 4. Elevation & Depth

We eschew traditional "Drop Shadows" in favor of **Tonal Layering**.

*   **The Layering Principle:** Place a `surface_container_lowest` (#000000) card inside a `surface_container_low` (#1e0831) container to create a "recessed" look. Conversely, stack `surface_container_highest` on `surface` for a natural lift.
*   **Ambient Shadows:** If a component must float (like a FAB or Tooltip), use an extra-diffused shadow: `box-shadow: 0 20px 40px rgba(0, 0, 0, 0.25)`. The shadow color should never be pure gray; it must be a deep violet tint derived from `on_primary_container`.
*   **Ghost Borders:** If accessibility requires a stroke, use `outline_variant` (#543f66) at **15% opacity**. This provides a "suggestion" of a boundary without breaking the soft, immersive aesthetic.

---

## 5. Components

### Buttons & Inputs
*   **Primary Button:** Uses `primary_container` (#b28cff) with `on_primary_container` (#2e006b) text. Shape: `xl` (3rem) rounded corners. 
*   **Secondary/Action Button:** `secondary` (#3adffa) with `on_secondary` (#004b56) text. These should be reserved for high-priority "Play" or "Action" triggers.
*   **Input Fields:** Use `surface_container_highest` (#34184c) with no border. The label sits in `label-md` using `on_surface_variant` (#bba1cf).

### Cards & Lists
*   **Forbid Dividers:** Do not use horizontal rules. Separate list items using `1.5rem` (md) vertical spacing or by alternating backgrounds between `surface_container` and `surface_container_low`.
*   **Corner Treatment:** All cards must use `lg` (2rem) or `xl` (3rem) corner radii.

### Signature Components
*   **Status Badges:** Use the `secondary_container` (#006877) for success and `error_container` (#a70138) for alerts, always with `full` (9999px) pill rounding.
*   **Icon Accents:** Paw prints and bones should be treated as "Pattern Elements." Set them in `outline` (#846c96) at 30% opacity, rotated at varying angles (15°, -10°) to create a sense of movement.

---

## 6. Do's and Don'ts

### Do
*   **Do** use extreme white space. Allow the deep purple `surface` to breathe.
*   **Do** overlap elements. Allow high-quality imagery (like a dog) to break the container of a card and "step out" onto the background.
*   **Do** use `secondary` (#3adffa) sparingly as a "spark" of energy against the dark violet.

### Don't
*   **Don't** use sharp 90-degree corners. Everything in this system must feel soft to the touch.
*   **Don't** use pure white (#FFFFFF) for body text; use `on_surface` (#f3deff) to reduce eye strain against the dark background.
*   **Don't** align icons to a rigid grid when used for decoration; "scatter" them to maintain the playful, organic brand personality.