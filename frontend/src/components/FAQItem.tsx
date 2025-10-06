import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import type { FAQItem } from "@/types";

interface FAQItemProps {
  faq: FAQItem;
  index?: number;
}

export function FAQItemComponent({ faq, index = 0 }: FAQItemProps) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-50px" }}
      transition={{ duration: 0.5, delay: index * 0.1 }}
      className="border border-border rounded-lg overflow-hidden transition-all hover:border-primary/50 hover:shadow-md"
    >
      <motion.button
        onClick={() => setIsOpen(!isOpen)}
        whileHover={{ backgroundColor: "rgba(var(--accent), 0.5)" }}
        className="w-full flex items-center justify-between p-4 text-left transition-colors"
      >
        <span className="font-medium pr-4">{faq.question}</span>
        <motion.svg
          animate={{ rotate: isOpen ? 180 : 0 }}
          transition={{ duration: 0.3 }}
          className="w-5 h-5 text-muted-foreground shrink-0"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </motion.svg>
      </motion.button>
      <AnimatePresence initial={false}>
        {isOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeInOut" }}
            className="overflow-hidden"
          >
            <div className="p-4 pt-0 text-sm text-muted-foreground">{faq.answer}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
