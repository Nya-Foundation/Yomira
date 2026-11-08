import { useT } from "../../lib/i18n";
import { Select } from "../ui/select";

/** Resting value for the batch picker, which acts as a menu rather than
 *  showing a current selection. */
const PLACEHOLDER = "__placeholder";

/**
 * Picks a download label, or none. Used per row to show and change that row's
 * label, and in the selection toolbar as a menu that assigns to everything
 * selected — hence `placeholder`, which keeps it from reading as a selection.
 */
export function LabelSelect({
  labels,
  value,
  onChange,
  disabled,
  placeholder,
}: {
  labels: string[];
  value?: string;
  onChange: (label: string | null) => void;
  disabled?: boolean;
  placeholder?: string;
}) {
  const t = useT();
  return (
    <Select
      value={placeholder ? PLACEHOLDER : value}
      onChange={(e) => {
        if (e.target.value === PLACEHOLDER) return;
        onChange(e.target.value === "" ? null : e.target.value);
      }}
      disabled={disabled}
      aria-label={t("downloads.assignLabel")}
      className="w-32"
    >
      {placeholder && (
        <option value={PLACEHOLDER} disabled>
          {placeholder}
        </option>
      )}
      <option value="">{t("downloads.noLabel")}</option>
      {labels.map((label) => (
        <option key={label} value={label}>
          {label}
        </option>
      ))}
    </Select>
  );
}
