import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { MailIcon, MessageIcon } from "@/components/ui/icons";

interface ContactCardProps {
  type: "email" | "community";
  email?: string;
  discord?: string;
  twitter?: string;
  index: number;
}

export function AnimatedContactCard({ type, email, discord, twitter, index }: ContactCardProps) {
  const isEmail = type === "email";

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.9 }}
      whileInView={{ opacity: 1, scale: 1 }}
      viewport={{ once: true, margin: "-50px" }}
      transition={{ duration: 0.5, delay: index * 0.2 }}
      whileHover={{ y: -8, scale: 1.02 }}
      className="p-6 rounded-lg border bg-card transition-all group"
    >
      <div className="flex flex-col items-center text-center space-y-4">
        <motion.div
          initial={{ scale: 0, rotate: -180 }}
          whileInView={{ scale: 1, rotate: 0 }}
          viewport={{ once: true }}
          transition={{
            type: "spring",
            stiffness: 200,
            delay: index * 0.2 + 0.2,
          }}
          className="p-4 rounded-full bg-primary/10 text-primary group-hover:bg-primary/20 transition-colors"
        >
          {isEmail ? (
            <MailIcon className="h-8 w-8" size={32} />
          ) : (
            <MessageIcon className="h-8 w-8" size={32} />
          )}
        </motion.div>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ delay: index * 0.2 + 0.3 }}
        >
          <h3 className="font-semibold mb-2">{isEmail ? "Email Support" : "Join our Community"}</h3>
          <p className="text-sm text-muted-foreground mb-4">
            {isEmail
              ? "Send us an email and we'll get back to you within 24 hours."
              : "Connect with other users and get help from the community."}
          </p>
          {isEmail ? (
            <motion.a
              href={`mailto:${email}`}
              whileHover={{ scale: 1.05 }}
              className="text-sm text-primary hover:underline"
            >
              {email}
            </motion.a>
          ) : (
            <div className="flex gap-3 justify-center">
              <motion.a
                href={discord}
                target="_blank"
                rel="noopener noreferrer"
                whileHover={{ scale: 1.1 }}
                whileTap={{ scale: 0.95 }}
              >
                <Button variant="outline" size="sm">
                  Discord
                </Button>
              </motion.a>
              <motion.a
                href={twitter}
                target="_blank"
                rel="noopener noreferrer"
                whileHover={{ scale: 1.1 }}
                whileTap={{ scale: 0.95 }}
              >
                <Button variant="outline" size="sm">
                  Twitter
                </Button>
              </motion.a>
            </div>
          )}
        </motion.div>
      </div>
    </motion.div>
  );
}
