import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { MessageSquare, Server, Code, ExternalLink } from 'lucide-react';
import { useEngineStore } from '../store/engineStore';
import styles from './FeedbackView.module.css';

const openUrl = (url: string) => invoke('open_url', { url });

export function FeedbackView() {
  const { language } = useEngineStore();

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.2, ease: 'easeOut' }}
      className={styles.container}
    >
      <header className={styles.header}>
        <h2 className={styles.title}>
          {language === 'tr' ? 'Geri Bildirim & Topluluk' : 'Feedback & Community'}
        </h2>
        <p className={styles.subtitle}>
          {language === 'tr' ? 'Topluluğumuza katılın, hata bildirin veya yeni özellikler talep edin.' : 'Join our community, report bugs, or request new features.'}
        </p>
      </header>

      <div className={styles.grid}>
        {/* Discord Profile */}
        <div className={styles.card}>
          <div className={styles.cardHeader}>
            <div className={`${styles.iconWrapper} ${styles.discordIcon}`}>
              <MessageSquare size={20} />
            </div>
            <div>
              <h3 className={styles.cardTitle}>{language === 'tr' ? 'Geliştirici' : 'Developer'}</h3>
              <p className={styles.cardSubtitle}>@Lulushu</p>
            </div>
          </div>
          <button
            className={`${styles.actionBtn} ${styles.discordBtn}`}
            onClick={() => openUrl('https://discord.com/users/852103749228036136')}
          >
            <ExternalLink size={14} /> {language === 'tr' ? 'Mesaj Gönder' : 'Send a Message'}
          </button>
        </div>

        {/* Discord Server */}
        <div className={styles.card}>
          <div className={styles.cardHeader}>
            <div className={`${styles.iconWrapper} ${styles.serverIcon}`}>
              <Server size={20} />
            </div>
            <div>
              <h3 className={styles.cardTitle}>{language === 'tr' ? 'Discord Topluluğu' : 'Discord Community'}</h3>
              <p className={styles.cardSubtitle}>{language === 'tr' ? 'Vane DPI Resmi Sunucusu' : 'Vane DPI Official Server'}</p>
            </div>
          </div>
          <button
            className={`${styles.actionBtn} ${styles.serverBtn}`}
            onClick={() => openUrl('https://discord.gg/luppux')}
          >
            <ExternalLink size={14} /> {language === 'tr' ? 'Sunucuya Katıl' : 'Join Server'}
          </button>
        </div>

        {/* GitHub Repository */}
        <div className={styles.card}>
          <div className={styles.cardHeader}>
            <div className={`${styles.iconWrapper} ${styles.githubIcon}`}>
              <Code size={20} />
            </div>
            <div>
              <h3 className={styles.cardTitle}>{language === 'tr' ? 'GitHub Deposu' : 'GitHub Repository'}</h3>
              <p className={styles.cardSubtitle}>{language === 'tr' ? 'Projeyi yıldızlayın & hata bildirin' : 'Star the project & report issues'}</p>
            </div>
          </div>
          <button
            className={styles.actionBtn}
            onClick={() => openUrl('https://github.com/lulushuz/Vane')}
          >
            <ExternalLink size={14} /> {language === 'tr' ? 'Depoyu Görüntüle' : 'View Repository'}
          </button>
        </div>
      </div>
    </motion.div>
  );
}
