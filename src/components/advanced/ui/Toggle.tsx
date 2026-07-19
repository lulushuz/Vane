import { motion } from 'framer-motion';
import styles from '../../../views/AdvancedView.module.css';

interface ToggleProps {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, disabled = false }: ToggleProps) {
  return (
    <div
      className={`${styles.toggle} ${checked ? styles.toggleActive : ''}`}
      onClick={() => { if (!disabled) onChange(!checked); }}
      aria-disabled={disabled}
      style={disabled ? { cursor: 'not-allowed', opacity: 0.45 } : undefined}
    >
      <motion.div
        className={styles.toggleKnob}
        layout
        transition={{ type: 'spring', stiffness: 500, damping: 30 }}
      />
    </div>
  );
}
