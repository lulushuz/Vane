import styles from '../../../views/AdvancedView.module.css';

interface NumberInputProps {
  value: number;
  min?: number;
  max?: number;
  onChange: (v: number) => void;
  disabled?: boolean;
}

export function NumberInput({ value, min = 1, max = 9999, onChange, disabled = false }: NumberInputProps) {
  return (
    <div className={styles.numInputContainer} aria-disabled={disabled} style={disabled ? { opacity: 0.45 } : undefined}>
      <button type="button" disabled={disabled} className={styles.numBtn} onClick={() => onChange(Math.max(min, value - 1))}>−</button>
      <div className={styles.numValue}>{value}</div>
      <button type="button" disabled={disabled} className={styles.numBtn} onClick={() => onChange(Math.min(max, value + 1))}>+</button>
    </div>
  );
}
